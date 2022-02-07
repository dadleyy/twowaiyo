use crate::constants;

#[derive(Debug, Clone)]
pub struct RedisConfig {
  pub(crate) host: String,
  pub(crate) port: String,
  pub(crate) password: String,
}

pub fn from_env() -> Option<RedisConfig> {
  let host = std::env::var(constants::REDIS_HOSTNAME_ENV).ok();
  let port = std::env::var(constants::REDIS_PORT_ENV).ok();
  let pass = std::env::var(constants::REDIS_PASSWORD_ENV).ok();

  host.zip(port).zip(pass).map(|((host, port), pass)| RedisConfig {
    host,
    port,
    password: pass,
  })
}
