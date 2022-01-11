use bankah::jobs::{JobError, TableAdminJob, TableJobOutput};
use bankah::state::{PlayerState, TableState};
use twowaiyo::{Player, Table};

fn stand_player(mut ts: TableState, mut ps: PlayerState) -> std::io::Result<(TableState, PlayerState)> {
  let mut player = Player::from(&ps);
  let table = Table::from(&ts);
  log::trace!("before player stand - {:?} {:?}", table, player);

  let table = table.stand(&mut player);
  let next = TableState::from(&table);

  if next.seats.contains_key(&ps.id) == false {
    ps.tables = ps.tables.drain(0..).filter(|id| id != &ts.id.to_string()).collect();
  }

  log::trace!("new player state - {:?}", player);

  ps.balance = player.balance;

  ts.seats = next
    .seats
    .into_iter()
    .filter_map(|(uuid, mut state)| {
      let original = ts.seats.remove(&uuid);

      original.map(|orig| {
        state.nickname = orig.nickname;
        (uuid, state)
      })
    })
    .collect();

  Ok((ts, ps))
}

pub async fn sit(services: &crate::Services, entities: &(String, String)) -> Result<TableJobOutput, JobError> {
  return Ok(TableJobOutput::BetStale);
}

pub async fn stand(services: &crate::Services, entities: &(String, String)) -> Result<TableJobOutput, JobError> {
  let (tid, pid) = entities;

  log::debug!("player '{}' is leaving table '{}'", pid, tid);

  let tables = services.tables();
  let players = services.players();

  let player = players
    .find_one(crate::db::doc! { "id": pid }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query player - {}", error);
      JobError::Terminal(format!("unable to query tables - {}", error))
    })?
    .ok_or_else(|| {
      log::warn!("unable to find player");
      JobError::Terminal(format!("unable to find player '{}'", pid))
    })?;

  let table = tables
    .find_one(crate::db::doc! { "id": tid }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      JobError::Terminal(format!("unable to query tables - {}", error))
    })?
    .ok_or_else(|| {
      log::warn!("unable to find table");
      JobError::Terminal(format!("unable to find table '{}'", tid))
    })?;

  let (table, player) = stand_player(table, player).map_err(|error| {
    log::warn!("unable to stand player - '{}'", error);
    JobError::Terminal(format!("logic error while standing player - '{}'", pid))
  })?;

  tables
    .replace_one(crate::db::doc! { "id": tid }, &table, None)
    .await
    .map_err(|error| {
      log::warn!("unable to persist table updates - {}", error);
      JobError::Retryable
    })?;

  log::debug!("table save, applying new player state for '{}'", player.id);

  players
    .update_one(
      crate::db::doc! { "id": player.id.to_string() },
      crate::db::doc! { "$set": { "balance": player.balance, "tables": player.tables } },
      None,
    )
    .await
    .map_err(|error| {
      log::warn!("unable to persist new player balance - {}", error);
      JobError::Retryable
    })?;

  log::debug!("player '{}' updated, reindexing populations", player.id);

  let job = bankah::jobs::TableJob::admin(TableAdminJob::ReindexPopulations);
  services.queue(&job).await.map_err(|error| {
    log::warn!("unable to queue reindexing job - {}", error);
    JobError::Terminal(format!("unable to queue reindex - {}", error))
  })?;

  return Ok(TableJobOutput::StandOk);
}

#[cfg(test)]
mod test {
  use super::stand_player;
  use bankah::state::{PlayerState, TableState};
  use twowaiyo::{Bet, Player, Table};

  #[test]
  fn test_stand_with_remaining() {
    let table = Table::with_dice(vec![2, 2].into_iter());
    let mut player = Player::with_balance(200);
    let table = table
      .sit(&mut player)
      .bet(&player, &Bet::start_pass(100))
      .unwrap()
      .roll()
      .table;

    let ps = PlayerState::from(&player);
    assert_eq!(ps.balance, 0);
    let (ts, ps) = stand_player(TableState::from(&table), ps).unwrap();
    assert_eq!(ps.balance, 100, "some balance was returned");
    assert_eq!(
      ts.seats.get(&ps.id).map(|s| s.balance),
      Some(0),
      "the balance was zeroed"
    );
    assert_eq!(
      ts.seats.get(&ps.id).map(|s| s.bets.len()),
      Some(1),
      "there is one bet left"
    );
  }
}
