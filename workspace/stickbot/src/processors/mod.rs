use bankah;
use twowaiyo;

use bankah::{BetFailureReason, TableJobOutput};
use twowaiyo::errors::{PlayerBetViolation, RuleViolation};

use crate::constants::{MONGO_DB_PLAYER_COLLECTION_NAME, MONGO_DB_TABLE_COLLECTION_NAME};
use crate::db;

fn failure_from_violation(violation: &RuleViolation) -> TableJobOutput {
  let inner = match violation {
    RuleViolation::PlayerBetViolation(PlayerBetViolation::InsufficientFunds) => BetFailureReason::InsufficientFunds,
    _ => BetFailureReason::Other,
  };

  TableJobOutput::BetFailed(inner)
}

pub async fn bet(services: &crate::Services, job: &bankah::BetJob) -> Result<bankah::TableJobOutput, bankah::JobError> {
  log::debug!("processing bet, services {:?}", services.status().await);

  let tables = services.collection::<bankah::TableState, _>(MONGO_DB_TABLE_COLLECTION_NAME);
  let players = services.collection::<bankah::PlayerState, _>(MONGO_DB_PLAYER_COLLECTION_NAME);

  let tstate = tables
    .find_one(db::doc! { "id": &job.table }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query for table - {}", error);
      bankah::JobError::Retryable
    })?
    .ok_or(bankah::JobError::Terminal("table-not-found".into()))?;

  log::debug!("loaded table state - {:?}", tstate);

  let pstate = players
    .find_one(db::doc! { "id": &job.player }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query for player - {}", error);
      bankah::JobError::Retryable
    })?
    .ok_or(bankah::JobError::Terminal("player-not-found".into()))?;

  log::debug!("loaded player state - {:?}", pstate);

  let table = twowaiyo::Table::from(&tstate);
  let player = twowaiyo::Player::from(&pstate);
  let bet = twowaiyo::Bet::from(&job.bet);

  if job.version != tstate.nonce {
    let id = &tstate.id;
    let current = &tstate.nonce;
    let attempt = &job.version;
    log::debug!("version discrep '{}' ('{}' vs '{}')", id, current, attempt);
    return Ok(bankah::TableJobOutput::BetStale);
  }

  let table = match table.bet(&player, &bet) {
    Err(inner) => {
      log::warn!("bet not valid - {:?}", inner);
      let result = failure_from_violation(&inner.error);
      return Ok(result);
    }
    Ok(updated) => {
      log::debug!("updated table with bet - {:?}", updated);
      updated
    }
  };

  let mut updated = bankah::TableState::from(&table);
  updated.nonce = tstate.nonce;

  tables
    .replace_one(db::doc! { "id": &tstate.id }, &updated, None)
    .await
    .map_err(|error| {
      log::warn!("unable to replace table state - {}", error);
      bankah::JobError::Retryable
    })?;

  log::info!("new table '{}': {:?}'", tstate.id, updated);

  Ok(bankah::TableJobOutput::BetProcessed)
}
