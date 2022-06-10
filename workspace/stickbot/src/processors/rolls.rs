use crate::db;

use bankah::jobs::{JobError, RollJob, TableJobOutput};
use bankah::state::{BetState, TableState};
use twowaiyo::Table;

fn apply_roll(mut state: TableState) -> Result<TableState, JobError> {
  let table = Table::from(&state);
  let mut rolled = table.roll();
  let mut next = TableState::from(&rolled.table);

  state.seats = state
    .seats
    .into_iter()
    .map(|(uuid, mut seat)| {
      let matching = next.seats.remove(&uuid).unwrap_or(seat.clone());
      let mut movement = rolled.results.remove(&uuid).unwrap_or_default();
      log::trace!("applying update for seat '{}' (moves {:?})", uuid, movement);
      let mut history = movement.map(|(bet, status, amount)| (BetState::from(&bet), status, amount));

      seat.balance = matching.balance;
      seat.bets = matching.bets;
      seat.history = seat.history.into_iter().chain(&mut history).collect();

      (uuid, seat)
    })
    .collect();

  state.button = next.button;
  state.nonce = uuid::Uuid::new_v4().to_string();
  state.roller = next.roller;
  state.rolls = next.rolls;

  Ok(state)
}

pub async fn roll(services: &crate::Services, job: &RollJob) -> Result<TableJobOutput, JobError> {
  let lookup = db::lookup_for_uuid(&job.table);

  let start = services
    .tables()
    .find_one(lookup, None)
    .await
    .map_err(|error| {
      log::warn!("failed table query - {}", error);
      JobError::Retryable
    })?
    .ok_or(JobError::Terminal("not-found".into()))?;

  log::debug!("attempting to process roll for table '{}'", start.id);

  if start.nonce != job.version {
    log::warn!("stale roll job (request '{}', current '{}')", job.version, start.nonce);
    return Ok(TableJobOutput::RollStale);
  }

  let updated = apply_roll(start)?;

  services
    .tables()
    .find_one_and_replace(db::lookup_for_uuid(&updated.id), &updated, None)
    .await
    .map_err(|error| {
      log::warn!("unable to replace updated table - {}", error);
      JobError::Terminal("failed-save".into())
    })?;

  Ok(TableJobOutput::RollProcessed)
}
