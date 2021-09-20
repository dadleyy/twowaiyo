use crate::web::{cookie as get_cookie, Error, Request, Result};

pub async fn create(request: Request) -> Result {
  get_cookie(&request);
  Ok("".into())
}
