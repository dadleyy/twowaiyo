use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use crate::db::doc;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

use bankah::state::{BetState, RaceType, TargetKind};

#[derive(Debug, Serialize)]
struct BetResult {
  job: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
struct BetPayload {
  kind: String,
  amount: u32,
  target: Option<u8>,
  table: uuid::Uuid,
  nonce: uuid::Uuid,
}

impl BetPayload {
  pub fn bet(&self) -> Option<BetState> {
    match self.kind.as_str() {
      "come" => Some(BetState::Race(RaceType::Come, self.amount, None)),
      "pass" => Some(BetState::Race(RaceType::Pass, self.amount, None)),
      "pass-odds" => Some(BetState::Target(TargetKind::PassOdds, self.amount, 0)),

      "come-odds" => self
        .target
        .map(|t| BetState::Target(TargetKind::ComeOdds, self.amount, t)),

      "field" => Some(BetState::Field(self.amount)),

      "hardway" => self
        .target
        .and_then(|raw| twowaiyo::Hardway::try_from(raw).ok().map(|_| raw))
        .map(|validated| BetState::Target(TargetKind::Hardway, self.amount, validated)),

      "place" => self.target.map(|t| BetState::Target(TargetKind::Place, self.amount, t)),

      _ => {
        log::warn!("unknown bet payload - {:?}", self);
        None
      }
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
  let tables = request.state().tables();
  let search = crate::db::lookup_for_uuid(&payload.table);

  log::debug!("attempting to place bet on table '{:?}': {:?}", payload.table, search);

  let state = tables
    .find_one(search, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      Error::from_str(500, "lookup")
    })?
    .ok_or_else(|| {
      log::warn!("bet attempted on invalid table {}", payload.table);
      Error::from_str(404, "no-table")
    })?;

  if state.nonce != payload.nonce {
    return Err(Error::from_str(422, "bad-version"));
  }

  log::info!("player '{}' making bet '{:?}', submitting job", player.id, bet);

  let job = bankah::jobs::TableJob::bet(bet, player.id.clone(), state.id.clone(), state.nonce.clone());

  request
    .state()
    .queue(&job)
    .await
    .map_err(|error| {
      log::warn!("unable to queue - {}", error);
      Error::from_str(500, "bad-queue")
    })
    .map(|id| BetResult { job: id })
    .and_then(|res| Body::from_json(&res))
    .map(|bod| Response::builder(200).body(bod).build())
}
