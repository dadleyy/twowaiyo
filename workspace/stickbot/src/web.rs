use http_types;
use tide;

pub use http_types::{Cookie, Url};
pub use tide::{Body, Error, Redirect, Response, Result};
pub type Request = tide::Request<crate::Services>;

pub fn cookie(request: &Request) -> Option<Cookie<'static>> {
  let session_cookie = request.cookie(crate::constants::STICKBOT_COOKIE_NAME)?;

  log::debug!("found cookie header - {session_cookie:?}");

  Some(session_cookie)
}
