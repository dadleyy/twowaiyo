use http_types;
use tide;

pub use http_types::{Cookie, Url};
pub use tide::{Body, Error, Redirect, Response, Result};
pub type Request = tide::Request<crate::Services>;

pub fn cookie(request: &Request) -> Option<Cookie<'static>> {
  request
    .header("Cookie")
    .and_then(|list| list.get(0))
    .map(|value| value.to_string())
    .and_then(|cook| Cookie::parse(cook).ok())
}
