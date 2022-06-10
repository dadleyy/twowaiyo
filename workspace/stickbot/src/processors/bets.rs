use bankah::jobs::{BetFailureReason, BetJob, JobError, TableJobOutput};
use bankah::state::{BetState, PlayerState, TableState};
use twowaiyo::errors::{PassLineNotEstablishedViolation, PlayerBetViolation, RuleViolation};

use crate::db;

fn failure_from_violation(violation: &RuleViolation) -> TableJobOutput {
  let inner = match violation {
    RuleViolation::PlayerBetViolation(PlayerBetViolation::InsufficientFunds) => BetFailureReason::InsufficientFunds,
    RuleViolation::PassLineNotEstablished(PassLineNotEstablishedViolation::ComeBet) => BetFailureReason::InvalidComeBet,
    _ => BetFailureReason::Other,
  };

  TableJobOutput::BetFailed(inner)
}

fn apply_bet(ps: PlayerState, mut ts: TableState, bs: BetState) -> Result<TableState, RuleViolation> {
  let player = twowaiyo::Player::from(&ps);
  let bet = twowaiyo::Bet::from(&bs);
  let table = twowaiyo::Table::from(&ts)
    .bet(&player, &bet)
    .map_err(|error| error.error)?;

  let mut next = TableState::from(&table);

  ts.nonce = uuid::Uuid::new_v4().to_string();
  ts.seats = ts
    .seats
    .into_iter()
    .map(|(uuid, mut seat)| {
      let other = next.seats.remove(&uuid);
      seat.balance = other.as_ref().map(|seat| seat.balance).unwrap_or(seat.balance);
      seat.bets = other.map(|seat| seat.bets).unwrap_or_default();
      (uuid, seat)
    })
    .collect();

  Ok(ts)
}

pub async fn bet<'a>(services: &crate::Services, job: &BetJob) -> Result<TableJobOutput, JobError> {
  let tables = services.tables();
  let players = services.players();

  log::trace!("processing bet job '{:?}'", job);
  let lookup = db::lookup_for_uuid(&job.table);

  let ts = tables
    .find_one(lookup, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query for table - {}", error);
      JobError::Retryable
    })?
    .ok_or_else(|| {
      log::warn!("unable to find table via {:?}", db::lookup_for_uuid(&job.table));
      JobError::Terminal("table-not-found".into())
    })?;

  // TODO: immediately after this check, we should set the nonce on the table to attmept at preventing race
  // conditions related to new bets happening during processing.
  if job.version != ts.nonce {
    log::warn!("skipping stale bet - {} {}", job.version, ts.nonce);
    return Ok(TableJobOutput::BetStale);
  }

  log::trace!("loaded table state - {:?}", ts);

  let ps = players
    .find_one(db::doc! { "id": job.player.to_string() }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query for player - {}", error);
      JobError::Retryable
    })?
    .ok_or_else(|| {
      log::warn!("unable to process bet, player not found: '{}'", job.player);
      JobError::Terminal("player-not-found".into())
    })?;

  log::trace!("loaded player state - {:?}", ps);

  // At this point we've loaded everything and only need to apply the logic. This is a failable operation, but for
  // logical reasons assocaited with the game, not so much the "system". If the processing fails here, the job is
  // still considered as "success", it just carries a failed bet.
  let next = match apply_bet(ps, ts, job.bet.clone()) {
    Err(violation) => return Ok(failure_from_violation(&violation)),
    Ok(next) => next,
  };

  tables
    .find_one_and_replace(db::lookup_for_uuid(&next.id), &next, None)
    .await
    .map_err(|error| {
      log::warn!("unable to replace table state - {}", error);
      JobError::Retryable
    })?;

  log::info!("new table '{}': {:?}'", next.id, next);

  Ok(TableJobOutput::BetProcessed)
}
