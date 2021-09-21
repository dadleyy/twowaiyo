use std::io::{Error, ErrorKind, Result};

use async_std::net::TcpStream;
use async_std::sync::Arc;
use async_std::sync::Mutex;

use crate::auth;
use crate::constants;
use crate::db;

#[derive(Clone)]
pub struct Services {
  db: db::Client,
  redis: Arc<Mutex<TcpStream>>,
}

impl Services {
  pub fn collection<T, N>(&self, name: N) -> db::Collection<T>
  where
    N: AsRef<str>,
  {
    let db = self.db.database(constants::MONGO_DB_DATABASE_NAME);
    db.collection::<T>(name.as_ref())
  }

  pub fn tables(&self) -> db::Collection<bankah::TableState> {
    self.collection(constants::MONGO_DB_TABLE_COLLECTION_NAME)
  }

  pub fn players(&self) -> db::Collection<bankah::PlayerState> {
    self.collection(constants::MONGO_DB_PLAYER_COLLECTION_NAME)
  }

  pub async fn queue(&self, job: &bankah::TableJob) -> Result<String> {
    let serialized = serde_json::to_string(&job).map_err(|error| {
      log::warn!("unable to serialize job - {}", error);
      Error::new(ErrorKind::Other, format!("{}", error))
    })?;

    let command = kramer::Command::List(kramer::ListCommand::Push(
      (kramer::Side::Right, kramer::Insertion::Always),
      constants::STICKBOT_BETS_QUEUE,
      kramer::Arity::One(serialized),
    ));

    self.command(&command).await.map(|_| job.id())
  }

  pub async fn authority<T>(&self, token: T) -> Option<auth::Authority>
  where
    T: std::fmt::Display,
  {
    let claims = auth::Claims::decode(&token).ok()?;
    let collection = self.collection::<bankah::PlayerState, _>(constants::MONGO_DB_PLAYER_COLLECTION_NAME);
    log::info!("decoded claims '{:?}', fetching user", claims);
    collection
      .find_one(db::doc! { "oid": claims.oid.clone(), "id": claims.id.clone() }, None)
      .await
      .ok()
      .and_then(|maybe_player| maybe_player.map(|player| auth::Authority::Player(player)))
  }

  pub async fn command<S, V>(&self, command: &kramer::Command<S, V>) -> Result<kramer::Response>
  where
    S: std::fmt::Display,
    V: std::fmt::Display,
  {
    let mut lock = self.redis.lock().await;
    let mut redis: &mut TcpStream = &mut lock;
    kramer::execute(&mut redis, command).await
  }

  pub async fn status(&self) -> Result<()> {
    let redis = self.redis.lock().await;
    redis.peer_addr().map(|addr| {
      log::debug!("addr - {:?}", addr);
      ()
    })
  }

  pub async fn new() -> Result<Self> {
    let mongo_url = std::env::var(constants::MONGO_DB_ENV_URL).unwrap_or_default();
    log::debug!("attempting to establish mongo connection at {}", mongo_url);
    let mongo = db::connect(mongo_url).await?;

    let redis_config = (
      std::env::var(constants::REDIS_HOSTNAME_ENV).unwrap_or_default(),
      std::env::var(constants::REDIS_PORT_ENV).unwrap_or_default(),
      std::env::var(constants::REDIS_PASSWORD_ENV).unwrap_or_default(),
    );
    log::debug!("redis configuration - '{}', connecting", redis_config.0);
    let mut redis = TcpStream::connect(format!("{}:{}", redis_config.0, redis_config.1)).await?;
    log::debug!("connection established - {:?}, authenticating", redis.peer_addr());

    if redis_config.2.len() > 0 {
      let cmd = kramer::Command::Auth::<String, String>(kramer::AuthCredentials::Password(redis_config.2));
      let result = kramer::execute(&mut redis, cmd).await?;
      log::debug!("authentication result - {:?}", result);
    }

    Ok(Services {
      db: mongo,
      redis: Arc::new(Mutex::new(redis)),
    })
  }
}
