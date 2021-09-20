use serde::{Deserialize, Serialize};

use crate::constants::{MONGO_DB_TABLE_COLLECTION_NAME, STICKBOT_BETS_QUEUE};
use crate::db::doc;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

#[derive(Debug, Serialize)]
struct BetResult {
  job: String,
}

#[derive(Debug, Deserialize)]
struct BetPayload {
  kind: String,
  amount: u32,
  table: String,
  nonce: String,
}

impl BetPayload {
  pub fn bet(&self) -> Option<bankah::BetState> {
    match self.kind.as_str() {
      "pass" => {
        log::debug!("building pass line bet");
        Some(bankah::BetState::Race(bankah::RaceType::Pass, self.amount, None))
      }
      _ => None,
    }
  }
}

pub async fn create(mut request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let payload = request.body_json::<BetPayload>().await?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  let bet = payload.bet().ok_or(Error::from_str(422, "bad-bet"))?;

  let tables = request
    .state()
    .collection::<bankah::TableState, _>(MONGO_DB_TABLE_COLLECTION_NAME);

  let state = tables
    .find_one(doc! { "id": &payload.table }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      Error::from_str(500, "lookup")
    })?
    .ok_or(Error::from_str(404, "no-table"))?;

  if state.nonce != payload.nonce {
    return Err(Error::from_str(422, "bad-version"));
  }

  let job = bankah::TableJob::bet(bet, player.id.clone(), state.id.clone(), state.nonce.clone());

  let serialized = serde_json::to_string(&job).map_err(|error| {
    log::warn!("unable to serialize bet - {}", error);
    error
  })?;

  let command = kramer::Command::List(kramer::ListCommand::Push(
    (kramer::Side::Left, kramer::Insertion::Always),
    STICKBOT_BETS_QUEUE,
    kramer::Arity::One(serialized),
  ));
  log::debug!("player '{:?}' making bet '{:?}'", player, job);

  request.state().command(&command).await.map_err(|error| {
    log::warn!("unable to persist job command - {}", error);
    Error::from_str(500, "bad-save")
  })?;

  log::info!("bet queued (job '{}')", job.id());

  Body::from_json(&BetResult { job: job.id() }).map(|body| Response::builder(200).body(body).build())
}
