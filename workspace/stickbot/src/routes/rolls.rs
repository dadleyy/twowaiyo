use serde::{Deserialize, Serialize};

use crate::db;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

#[derive(Debug, Serialize)]
pub struct RollResponse {
  job: String,
}

#[derive(Debug, Deserialize)]
pub struct RollPayload {
  table: uuid::Uuid,
  nonce: uuid::Uuid,
}

pub async fn create(mut request: Request) -> Result {
  let payload = request.body_json::<RollPayload>().await?;
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, crate::constants::EMPTY_RESPONSE))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, crate::constants::EMPTY_RESPONSE))?;

  log::debug!("player '{}' to roll on table '{}'", player.id, payload.table);

  let lookup = db::lookup_for_uuid(&payload.table);
  let table = request
    .state()
    .tables()
    .find_one(lookup, None)
    .await?
    .ok_or(Error::from_str(404, "not-found"))?;

  if table.roller != Some(player.id) {
    return Err(Error::from_str(404, "not-found"));
  }

  // Update the nonce/version of the table before submitting our job.
  let nonce = uuid::Uuid::new_v4();
  let next = bankah::state::TableState {
    nonce: nonce.clone(),
    ..table
  };

  log::debug!("found table '{}' setting new nonce '{}'", next.id, nonce);

  request
    .state()
    .tables()
    .find_one_and_replace(db::lookup_for_uuid(&payload.table), &next, None)
    .await
    .map_err(|error| {
      log::warn!("unable to update nonce of table - {}", error);
      error
    })?;

  let job = bankah::jobs::TableJob::roll(next.id.clone(), nonce);

  request
    .state()
    .queue(&job)
    .await
    .map_err(|error| {
      log::warn!("unable to queue job - {}", error);
      Error::from_str(500, "bad-queue")
    })
    .map(|id| RollResponse { job: id.to_string() })
    .and_then(|res| Body::from_json(&res))
    .map(|bod| Response::builder(200).body(bod).build())
}
