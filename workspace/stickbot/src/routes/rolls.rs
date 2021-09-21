use crate::web::{cookie as get_cookie, Error, Request, Result};

pub async fn create(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  log::debug!("player '{}' attempting to roll", player.id);

  Ok("".into())
}
