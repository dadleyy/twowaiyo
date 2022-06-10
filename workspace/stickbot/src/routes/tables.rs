use async_std::stream::StreamExt;
use serde::{Deserialize, Serialize};

use crate::constants;
use crate::db::doc;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

use bankah::jobs::TableJob;
use bankah::state::PlayerState;

#[derive(Debug, Serialize)]
enum JoinFailure {
  TooManyActiveTables,
  InsufficientFunds,
}

fn join_failure(ps: &PlayerState) -> Option<JoinFailure> {
  let max = std::env::var(constants::STICKBOT_MAX_ACTIVE_TABLES_PER_PLAYER_ENV)
    .ok()
    .and_then(|v| v.parse::<usize>().ok())
    .unwrap_or(constants::STICKBOT_DEFAULT_MAX_ACTIVE_TABLES_PER_PLAYER);

  if ps.tables.len() >= max {
    return Some(JoinFailure::TooManyActiveTables);
  }

  if (ps.balance > 0) != true {
    return Some(JoinFailure::InsufficientFunds);
  }

  return None;
}

// ## Route
// List tables.
pub async fn list(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  log::trace!("listing tables for '{:?}'", player);
  let collection = request.state().table_index();

  let mut tables = collection.find(None, None).await.map_err(|error| {
    log::warn!("unable to query tables - {}", error);
    Error::from_str(500, "load-tables")
  })?;

  let mut page = Vec::with_capacity(10);

  while let Some(doc) = tables.next().await {
    if let Ok(state) = doc {
      page.push(state)
    }

    if page.len() >= 10 {
      break;
    }
  }

  Body::from_json(&page).map(|body| Response::builder(200).body(body).build())
}

#[derive(Debug, Deserialize)]
struct TableActionPayload {
  pub id: String,
}

// ## Route
// Get all information about a specific table
pub async fn find(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  let query = request.query::<TableActionPayload>().map_err(|error| {
    log::warn!("invalid lookup - {}", error);
    Error::from_str(404, "not-found")
  })?;

  log::info!("[info] looking up table '{}'", query.id);

  let table = request
    .state()
    .tables()
    .find_one(crate::db::doc! { "id": &query.id }, None)
    .await
    .map_err(|error| {
      log::warn!("[info] unable to perform lookup - {}", error);
      Error::from_str(500, "bad service")
    })?
    .ok_or_else(|| {
      log::warn!("unable to find table {:?}", crate::db::lookup_for_uuid(&query.id));
      Error::from_str(404, "no-table")
    })?;

  Body::from_json(&table).map(|body| Response::builder(200).body(body).build())
}

// ## Route
// Joins a table.
pub async fn join(mut request: Request) -> Result {
  let query = request.body_json::<TableActionPayload>().await?;
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, ""))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|auth| auth.player())
    .ok_or(Error::from_str(404, "no-player"))?;

  if let Some(reason) = join_failure(&player) {
    let body = Body::from_string(format!("{:?}", reason));
    return Ok(Response::builder(422).body(body).build());
  }

  let job = TableJob::sit(query.id.to_string(), player.id.to_string());
  let id = request.state().queue(&job).await.map_err(|error| {
    log::warn!("unable to queue sit job - {}", error);
    error
  })?;
  let res = bankah::JobResponse { job: id, output: None };
  Body::from_json(&res).map(|body| Response::builder(200).body(body).build())
}

// ## Route
// Creates a new table and sits the player.
pub async fn create(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, ""))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|auth| auth.player())
    .ok_or(Error::from_str(404, "no-player"))?;

  if let Some(reason) = join_failure(&player) {
    let body = Body::from_string(format!("{:?}", reason));
    return Ok(Response::builder(422).body(body).build());
  }

  let job = TableJob::Create(bankah::jobs::JobWapper::wrap(player.id.to_string()));
  let id = request.state().queue(&job).await.map_err(|error| {
    log::warn!("unable to queue table creation job - '{}'", error);
    error
  })?;
  let res = bankah::JobResponse { job: id, output: None };
  Body::from_json(&res).map(|body| Response::builder(200).body(body).build())
}

// ## Route
// Leave table.
pub async fn leave(mut request: Request) -> Result {
  let query = request.body_json::<TableActionPayload>().await.map_err(|error| {
    log::warn!("invalid table exit payload - {}", error);
    error
  })?;
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "unauth"))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .ok_or(Error::from_str(404, "no-user"))?
    .player()
    .ok_or(Error::from_str(404, "no-player"))?;

  log::debug!("user '{}' leaving table '{}'", player.id, query.id);
  let job = TableJob::stand(query.id.to_string(), player.id.to_string());

  let id = request.state().queue(&job).await.map_err(|error| {
    log::warn!("unable to queue stand job - {}", error);
    error
  })?;

  log::debug!("job '{}' queued", id);

  let res = bankah::JobResponse { job: id, output: None };
  Body::from_json(&res).map(|body| Response::builder(200).body(body).build())
}
