use serde::{Deserialize, Serialize};

use crate::db;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

#[derive(Debug, Serialize)]
pub struct RollResponse {
  job: String,
}

#[derive(Debug, Deserialize)]
pub struct RollPayload {
  table: String,
  nonce: String,
}

pub async fn create(mut request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let payload = request.body_json::<RollPayload>().await?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  log::debug!("player '{}' to roll on table '{}'", player.id, payload.table);

  let table = request
    .state()
    .tables()
    .find_one(db::doc! { "id": &payload.table }, None)
    .await?
    .ok_or(Error::from_str(404, "not-found"))?;

  if table.roller != Some(player.id) {
    return Err(Error::from_str(404, "not-found"));
  }

  let nonce = uuid::Uuid::new_v4().to_string();
  let next = bankah::TableState {
    nonce: nonce.clone(),
    ..table
  };

  log::debug!("found table '{}' setting new nonce '{}'", next.id, nonce);

  request
    .state()
    .tables()
    .replace_one(db::doc! { "id": &payload.table }, &next, None)
    .await
    .map_err(|error| {
      log::warn!("unable to update nonce of table - {}", error);
      error
    })?;

  let job = bankah::TableJob::roll(next.id.clone(), nonce);

  request
    .state()
    .queue(&job)
    .await
    .map_err(|error| {
      log::warn!("unable to queue job - {}", error);
      Error::from_str(500, "bad-queue")
    })
    .map(|id| RollResponse { job: id })
    .and_then(|res| Body::from_json(&res))
    .map(|bod| Response::builder(200).body(bod).build())
}
