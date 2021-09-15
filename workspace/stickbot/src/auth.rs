use serde::{Deserialize, Serialize};

use crate::constants;

#[derive(Debug, Deserialize, Serialize)]
pub struct UserInfo {
  sub: String,
  nickname: String,
  picture: String,
}

pub async fn from_token<T>(token: T) -> Option<UserInfo>
where
  T: std::fmt::Display,
{
  let uri = std::env::var(constants::AUTH_O_USERINFO_URI_ENV).unwrap_or_default();

  let mut res = surf::get(&uri)
    .header("Authorization", format!("Bearer {}", token))
    .await
    .ok()?;

  if res.status() != surf::StatusCode::Ok {
    log::warn!("bad response status - '{:?}'", res.status());
    return None;
  }

  log::debug!("loaded info with status '{}', attempting to parse", res.status());
  res.body_json::<UserInfo>().await.ok()
}
