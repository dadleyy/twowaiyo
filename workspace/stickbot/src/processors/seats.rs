use bankah::jobs::{JobError, TableAdminJob, TableJobOutput};
use bankah::state::{PlayerState, TableState};
use twowaiyo::{Player, Table};

async fn find_player(services: &crate::Services, id: &String) -> Result<PlayerState, JobError> {
  services
    .players()
    .find_one(crate::db::doc! { "id": id }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query player - {}", error);
      JobError::Terminal(format!("unable to query players - {}", error))
    })?
    .ok_or_else(|| {
      log::warn!("unable to find player");
      JobError::Terminal(format!("unable to find player '{}'", id))
    })
}

fn sit_player(mut ts: TableState, mut ps: PlayerState) -> std::io::Result<(TableState, PlayerState)> {
  let mut player = Player::from(&ps);

  let table = Table::from(&ts).sit(&mut player);
  let next = TableState::from(&table);

  ps.balance = player.balance;

  ps.tables = ps.tables.drain(0..).chain(Some(ts.id.to_string())).collect();

  ts.roller = next.roller;

  ts.seats = next
    .seats
    .into_iter()
    .map(|(uuid, seat)| {
      let original = ts.seats.remove(&uuid);
      let mut current = original.unwrap_or(seat);

      let nickname = match uuid == ps.id {
        true => ps.nickname.clone(),
        false => current.nickname.clone(),
      };

      current.nickname = nickname;

      (uuid, current)
    })
    .collect();

  Ok((ts, ps))
}

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
  ts.roller = next.roller;

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

pub async fn create(services: &crate::Services, pid: &String) -> Result<TableJobOutput, JobError> {
  let player = find_player(&services, &pid).await?;
  let name = crate::names::generate().map_err(|error| {
    log::warn!("unable to generate random name - {}", error);
    JobError::Retryable
  })?;
  let blank = TableState::with_name(name);
  log::debug!("creating blank table - {:?}", blank);
  let (table, player) = sit_player(blank, player).map_err(|error| {
    log::warn!("logic error while sitting player '{}' at new table - {}", pid, error);
    JobError::Terminal("".into())
  })?;

  let query = crate::db::doc! { "id": pid };
  let updates = crate::db::doc! { "$set": { "balance": player.balance, "tables": player.tables } };
  let opts = crate::db::FindOneAndUpdateOptions::builder()
    .return_document(crate::db::ReturnDocument::After)
    .build();

  services
    .players()
    .find_one_and_update(query, updates, opts)
    .await
    .map_err(|error| {
      log::warn!("unable to update player balance after create - {}", error);
      JobError::Terminal(format!("failed player balance update - {}", error))
    })?
    .ok_or_else(|| {
      log::warn!("no player to update");
      JobError::Terminal(format!("missing player '{}' during balance update", pid))
    })?;

  log::debug!("inserting new table {:?}", table);

  // TODO(mongo-uuid): we're using a `find_one_and_replace` w/ the upsert call here to circumvent the serialization
  // discrepency between insertion and find/replace methods on the mongodb driver collection.
  let tops = crate::db::FindOneAndReplaceOptions::builder().upsert(true).build();
  services
    .tables()
    .find_one_and_replace(crate::db::lookup_for_uuid(&table.id), &table, Some(tops))
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      JobError::Terminal(format!("missing player '{}' during balance update", pid))
    })?;

  // TODO: do we care if our attempt to enqueue a job fails from the web thread?
  services
    .queue(&bankah::jobs::TableJob::admin(TableAdminJob::ReindexPopulations))
    .await
    .map(|_| ())
    .unwrap_or_else(|error| {
      log::warn!("unable to queue indexing job - {}", error);
      ()
    });

  Ok(TableJobOutput::TableCreated(table.id.to_string()))
}

pub async fn sit(services: &crate::Services, entities: &(String, String)) -> Result<TableJobOutput, JobError> {
  let (tid, pid) = entities;
  log::debug!("player '{}' is sitting down to table '{}'", pid, tid);

  let tables = services.tables();
  let players = services.players();

  let player = players
    .find_one(crate::db::doc! { "id": pid }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to query player - {}", error);
      JobError::Terminal(format!("unable to query players - {}", error))
    })?
    .ok_or_else(|| {
      log::warn!("unable to find player");
      JobError::Terminal(format!("unable to find player '{}'", pid))
    })?;

  let state = tables
    .find_one(crate::db::doc! { "id": tid }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      JobError::Terminal(format!("unable to query tables - {}", error))
    })?
    .ok_or_else(|| {
      log::warn!("unable to find table '{}'", tid);
      JobError::Terminal(format!("unable to find table '{}'", tid))
    })?;

  let (ts, ps) = sit_player(state, player).map_err(|error| {
    log::warn!("logic error sitting player - {}", error);
    JobError::Terminal(format!("unable to sit player '{}'", error))
  })?;

  let opts = crate::db::FindOneAndUpdateOptions::builder()
    .return_document(crate::db::ReturnDocument::After)
    .build();

  // TODO(player-id): another example of player id peristence mismatch.
  players
    .find_one_and_update(
      crate::db::doc! { "id": pid },
      crate::db::doc! { "$set": { "balance": ps.balance, "tables": ps.tables } },
      opts,
    )
    .await
    .map_err(|error| {
      log::warn!("unable to update player balance after join - {}", error);
      JobError::Terminal(format!("unable to update player '{}'", error))
    })?
    .ok_or_else(|| {
      log::warn!("no player balance updated");
      JobError::Terminal(format!("player '{}' not found, no update applied", pid))
    })?;

  tables
    .find_one_and_replace(crate::db::doc! { "id": tid }, &ts, None)
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      JobError::Terminal(format!("table '{}' not updated - {}", tid, error))
    })?;

  // TODO: do we care if our attempt to enqueue a job fails from the web thread?
  let job = bankah::jobs::TableJob::admin(TableAdminJob::ReindexPopulations);
  services.queue(&job).await.map(|_| ()).unwrap_or_else(|error| {
    log::warn!("unable to queue indexing job - {}", error);
    ()
  });

  log::info!("player joined table '{}'", ts.id);

  return Ok(TableJobOutput::SitOk);
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

  if table.seats.len() == 0 {
    log::debug!("table '{}' is now empty, deleting", table.id);

    tables
      .delete_one(crate::db::doc! { "id": tid }, None)
      .await
      .map_err(|error| {
        log::warn!("unable to persist table updates - {}", error);
        JobError::Retryable
      })?;

    services
      .table_index()
      .delete_one(crate::db::doc! { "id": tid }, None)
      .await
      .map_err(|error| {
        log::warn!("unable to persist table updates - {}", error);
        JobError::Retryable
      })?;

    log::debug!("table '{}' cleanup complete", tid);

    return Ok(TableJobOutput::FinalStandOk);
  } else {
    tables
      .replace_one(crate::db::doc! { "id": tid }, &table, None)
      .await
      .map_err(|error| {
        log::warn!("unable to persist table updates - {}", error);
        JobError::Retryable
      })?;
  }

  log::debug!("table save, applying new player state for '{}'", player.id);

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
