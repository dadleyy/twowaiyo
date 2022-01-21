use std::io::{Error, ErrorKind, Result};

use async_std::net::TcpStream;
use async_std::sync::Arc;
use async_std::sync::Mutex;

use crate::auth;
use crate::constants;
use crate::db;

async fn connect_redis() -> Result<TcpStream> {
  let config = (
    std::env::var(constants::REDIS_HOSTNAME_ENV).unwrap_or_default(),
    std::env::var(constants::REDIS_PORT_ENV).unwrap_or_default(),
    std::env::var(constants::REDIS_PASSWORD_ENV).unwrap_or_default(),
  );

  log::debug!("redis configuration - '{}', connecting", config.0);
  let mut redis = TcpStream::connect(format!("{}:{}", config.0, config.1)).await?;
  log::debug!("connection established - {:?}, authenticating", redis.peer_addr());

  if config.2.len() > 0 {
    let cmd = kramer::Command::Auth::<String, String>(kramer::AuthCredentials::Password(config.2));
    let result = kramer::execute(&mut redis, cmd).await?;
    log::debug!("authentication result - {:?}", result);
  }

  Ok(redis)
}

fn response_string(response: &kramer::ResponseValue) -> Option<String> {
  match response {
    kramer::ResponseValue::String(inner) => Some(inner.clone()),
    res => {
      log::warn!("strange response from job queue - {:?}", res);
      None
    }
  }
}

fn parse_pop(response: &kramer::ResponseValue) -> Option<bankah::jobs::TableJob> {
  response_string(&response).and_then(|contents| serde_json::from_str::<bankah::jobs::TableJob>(&contents).ok())
}

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

  pub fn table_index(&self) -> db::Collection<bankah::state::TableIndexState> {
    self.collection(&format!("{}", crate::env::Collection::TableList))
  }

  pub fn tables(&self) -> db::Collection<bankah::state::TableState> {
    self.collection(&format!("{}", crate::env::Collection::Tables))
  }

  pub fn players(&self) -> db::Collection<bankah::state::PlayerState> {
    self.collection(&format!("{}", crate::env::Collection::Players))
  }

  pub async fn pop(&self) -> Result<Option<bankah::jobs::TableJob>> {
    let cmd = kramer::Command::List::<_, String>(kramer::ListCommand::Pop(
      kramer::Side::Left,
      crate::env::JobStore::Queue,
      Some((None, 3)),
    ));

    let result = match self.command(&cmd).await {
      Err(error) => {
        log::warn!("unable to pop from bet queue - {}", error);
        return Err(Error::new(ErrorKind::Other, format!("{}", error)));
      }
      Ok(kramer::Response::Item(kramer::ResponseValue::Empty)) => {
        log::debug!("empty response from queue");
        return Ok(None);
      }
      Ok(kramer::Response::Array(values)) => values,
      Ok(kramer::Response::Error) => {
        log::warn!("unable to pop from queue - redis error");
        return Err(Error::new(ErrorKind::Other, "invalid-response"));
      }
      Ok(kramer::Response::Item(inner)) => {
        log::warn!("unknown response from pop - '{:?}'", inner);
        return Err(Error::new(ErrorKind::Other, format!("{:?}", inner)));
      }
    };

    log::debug!("result from pop - {:?}, attempting to deserialize", result);

    Ok(result.get(1).and_then(parse_pop))
  }

  pub async fn queue(&self, job: &bankah::jobs::TableJob) -> Result<uuid::Uuid> {
    let serialized = serde_json::to_string(&job).map_err(|error| {
      log::warn!("unable to serialize job - {}", error);
      Error::new(ErrorKind::Other, format!("{}", error))
    })?;

    let command = kramer::Command::List(kramer::ListCommand::Push(
      (kramer::Side::Right, kramer::Insertion::Always),
      crate::env::JobStore::Queue,
      kramer::Arity::One(serialized),
    ));

    self.command(&command).await.map(|result| {
      log::debug!("executed queue command - {:?}", result);
      job.id()
    })
  }

  pub async fn authority<T>(&self, token: T) -> Option<auth::Authority>
  where
    T: std::fmt::Display,
  {
    let token = format!("{}", token);
    let cmd = kramer::Command::Hashes::<&str, &str>(kramer::HashCommand::Get(
      constants::STICKBOT_SESSION_STORE,
      Some(kramer::Arity::One(&token)),
    ));

    // TODO: should the storage value be used? is a hash the right move here?
    match self.command(&cmd).await.ok()? {
      kramer::Response::Item(kramer::ResponseValue::String(inner)) => Some(inner),
      other => {
        log::trace!("session-store lookup missing or invalid - {:?}", other);
        None
      }
    }?;

    let claims = auth::Claims::decode(&token).ok()?;
    let collection = self.players();
    let admins = std::env::var(constants::STICKBOT_ADMIN_EMAILS_ENV).unwrap_or_default();

    log::trace!("decoded claims '{:?}', fetching user (admins {})", claims, admins);

    // TODO(player-id): the player id is serialized as a string when peristing into the players collection during the
    // completion of the oauth flow.
    collection
      .find_one(db::doc! { "oid": claims.oid.clone(), "id": claims.id.clone() }, None)
      .await
      .ok()
      .and_then(|maybe_player| maybe_player)
      .map(
        |player| match player.emails.iter().find(|e| e.as_str() == admins.as_str()) {
          Some(_) => auth::Authority::Admin(player),
          None => auth::Authority::Player(player),
        },
      )
  }

  pub async fn command<S, V>(&self, command: &kramer::Command<S, V>) -> Result<kramer::Response>
  where
    S: std::fmt::Display,
    V: std::fmt::Display,
  {
    self.inner_command(command, 0).await
  }

  async fn inner_command<S, V>(&self, command: &kramer::Command<S, V>, mut attempt: u8) -> Result<kramer::Response>
  where
    S: std::fmt::Display,
    V: std::fmt::Display,
  {
    loop {
      log::debug!("requesting tcp write access trhough lock");
      let mut lock = self.redis.lock().await;
      let mut redis: &mut TcpStream = &mut lock;
      log::debug!("lock acquired, attempting to send command");
      let result = kramer::execute(&mut redis, command).await;

      if attempt > 10 {
        log::warn!("retry-attempts exceeded maximum, returning result {:?}", result);
        return result;
      }

      if let Err(error) = &result {
        log::warn!("failed executing command - {}", error);

        if error.kind() == ErrorKind::BrokenPipe && attempt < 10 {
          log::info!("broken pipe, attempting to re-establish connection");
          *lock = connect_redis().await?;
        }

        attempt = attempt + 1;
      }

      if let Ok(response) = result {
        log::debug!("redis command executed successfully - {:?}", response);
        return Ok(response);
      }
    }
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
    let redis = connect_redis().await?;

    Ok(Services {
      db: mongo,
      redis: Arc::new(Mutex::new(redis)),
    })
  }
}
