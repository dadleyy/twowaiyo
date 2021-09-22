use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

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
  target: Option<u8>,
  table: String,
  nonce: String,
}

impl BetPayload {
  pub fn bet(&self) -> Option<bankah::BetState> {
    match self.kind.as_str() {
      "come" => Some(bankah::BetState::Race(bankah::RaceType::Come, self.amount, None)),
      "pass" => Some(bankah::BetState::Race(bankah::RaceType::Pass, self.amount, None)),
      "pass-odds" => Some(bankah::BetState::Target(bankah::TargetKind::PassOdds, self.amount, 0)),

      "come-odds" => self
        .target
        .map(|t| bankah::BetState::Target(bankah::TargetKind::ComeOdds, self.amount, t)),

      "field" => Some(bankah::BetState::Field(self.amount)),

      "hardway" => self
        .target
        .and_then(|raw| twowaiyo::Hardway::try_from(raw).ok().map(|_| raw))
        .map(|validated| bankah::BetState::Target(bankah::TargetKind::Hardway, self.amount, validated)),

      "place" => self
        .target
        .map(|t| bankah::BetState::Target(bankah::TargetKind::Place, self.amount, t)),

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

  log::info!("player '{}' making bet '{:?}', submitting job", player.id, bet);

  let job = bankah::TableJob::bet(bet, player.id.clone(), state.id.clone(), state.nonce.clone());

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
