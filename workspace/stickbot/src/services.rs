use std::io::{Error, ErrorKind, Result};

use async_std::net::TcpStream;
use async_std::sync::Arc;
use async_std::sync::Mutex;

use crate::auth;
use crate::constants;
use crate::db;

async fn connect_redis(config: &crate::redis::RedisConfig) -> Result<TcpStream> {
  log::debug!("redis configuration - '{}', connecting", config.host);
  let mut redis = TcpStream::connect(format!("{}:{}", config.host, config.port)).await?;
  log::debug!("connection established - {:?}, authenticating", redis.peer_addr());

  if config.password.len() > 0 {
    let cmd = kramer::Command::Auth::<&String, &String>(kramer::AuthCredentials::Password(&config.password));
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
  rc: crate::redis::RedisConfig,
  version: String,
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

  pub async fn queue(&self, job: &bankah::jobs::TableJob) -> Result<String> {
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
    match self
      .command(&cmd)
      .await
      .map_err(|error| {
        log::warn!("unable to issue redis session command - {error}");
        error
      })
      .ok()?
    {
      kramer::Response::Item(kramer::ResponseValue::String(inner)) => Some(inner),
      other => {
        log::warn!("session-store lookup missing or invalid - {:?}", other);
        None
      }
    }?;

    let claims = auth::Claims::decode(&token).ok()?;
    let collection = self.players();
    let admins = std::env::var(constants::STICKBOT_ADMIN_EMAILS_ENV).unwrap_or_default();

    log::debug!(
      "decoded claims exp:'{}', oid:'{}', id:'{:?}'",
      claims.exp,
      claims.oid,
      claims.id
    );

    // TODO(player-id): the player id is serialized as a string when peristing into the players collection during the
    // completion of the oauth flow.
    collection
      .find_one(db::doc! { "oid": claims.oid, "id": claims.id}, None)
      .await
      .map_err(|error| {
        log::warn!("unable to query player collection - {error}");
        error
      })
      .ok()
      .or_else(|| {
        log::warn!("unable to find player matching query...");
        None
      })
      .and_then(|maybe_player| maybe_player)
      .or_else(|| {
        log::warn!("[mongo-problem?] unable to make player from query result.");
        None
      })
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

  #[allow(unused_assignments)]
  async fn inner_command<S, V>(&self, command: &kramer::Command<S, V>, mut attempt: u8) -> Result<kramer::Response>
  where
    S: std::fmt::Display,
    V: std::fmt::Display,
  {
    loop {
      if attempt > 10 {
        log::warn!("failed redis connection after {} attempts", attempt);
        return Err(Error::new(ErrorKind::Other, "too-many-attempts"));
      }

      log::debug!("requesting tcp write access through lock (attempt {})", attempt);
      let mut lock = self.redis.lock().await;
      let mut redis: &mut TcpStream = &mut lock;
      log::debug!("lock acquired, attempting to send command");

      match async_std::future::timeout(std::time::Duration::from_secs(5), kramer::execute(&mut redis, command)).await {
        Err(timeout_error) => {
          log::warn!("timeout error during command transfer - {}", timeout_error);
          *lock = connect_redis(&self.rc).await?;
          attempt += 1;
          return Err(Error::new(ErrorKind::Other, "timeout-error"));
        }
        Ok(Err(error)) => {
          log::warn!("failed executing command - {}", error);

          if error.kind() == ErrorKind::BrokenPipe {
            log::warn!("broken pipe, attempting to re-establish connection");
            *lock = connect_redis(&self.rc).await?;
          }

          attempt += 1;
        }
        Ok(Ok(response)) => {
          log::debug!("redis success - {:?} (attempt {})", response, attempt);
          return Ok(response);
        }
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
    let mc = std::env::var(constants::MONGO_DB_ENV_URL).map_err(|error| {
      log::warn!("unable to find mongo config '{}' in environment", error);
      Error::new(ErrorKind::Other, "missing-redis-config")
    })?;
    let rc = crate::redis::from_env().ok_or(Error::new(ErrorKind::Other, "missing-redis-config"))?;

    log::info!("connecting to mongo...");
    let mongo = db::connect(mc).await?;

    log::info!("connecting to redis...");
    let redis = connect_redis(&rc).await?;

    log::info!("services ready!");
    Ok(Services {
      db: mongo,
      rc: rc,
      redis: Arc::new(Mutex::new(redis)),
      version: std::option_env!("TWOWAIYO_VERSION").unwrap_or("dev").to_string(),
    })
  }
}

impl std::fmt::Display for Services {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "stickbot-services@{}", self.version)
  }
}
