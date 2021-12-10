use async_std::stream::StreamExt;
use serde::Deserialize;

use crate::db::{doc, FindOneAndReplaceOptions, FindOneAndUpdateOptions, ReturnDocument};
use crate::names;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

use bankah::state::{PlayerState, TableState};
use twowaiyo::{Player, Table};

// During conversion between our `twowaiyo` engine types, sever fields will be lost that need to be updated with the
// correct state.
fn sit_player(mut ts: TableState, mut ps: PlayerState) -> Result<(TableState, PlayerState)> {
  let mut player = Player::from(&ps);

  let table = Table::from(&ts).sit(&mut player);
  let next = TableState::from(&table);

  ps.balance = player.balance;
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

fn stand_player(mut ts: TableState, mut ps: PlayerState) -> Result<(TableState, PlayerState)> {
  let mut player = Player::from(&ps);
  let table = Table::from(&ts);
  log::trace!("before player stand - {:?} {:?}", table, player);

  let table = table.stand(&mut player);
  let next = TableState::from(&table);

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
  pub id: uuid::Uuid,
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

  log::trace!("looking up table {}", query.id);

  let table = request
    .state()
    .tables()
    .find_one(crate::db::lookup_for_uuid(&query.id), None)
    .await
    .map_err(|error| {
      log::warn!("unable to perform lookup - {}", error);
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

  if player.balance == 0 {
    let body = Body::from_string("no-balance".into());
    return Ok(Response::builder(422).body(body).build());
  }

  let tables = request.state().tables();
  let players = request.state().players();
  let lookup = crate::db::lookup_for_uuid(&query.id);

  let state = tables
    .find_one(lookup, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      Error::from_str(500, "lookup")
    })?
    .ok_or_else(|| {
      log::warn!("unable to find table {:?}", crate::db::lookup_for_uuid(&query.id));
      Error::from_str(404, "no-table")
    })?;

  let (ts, ps) = sit_player(state, player)?;

  let opts = FindOneAndUpdateOptions::builder()
    .return_document(ReturnDocument::After)
    .build();

  // TODO(player-id): another example of player id peristence mismatch.
  players
    .find_one_and_update(
      doc! { "id": ps.id.to_string() },
      doc! { "$set": { "balance": ps.balance } },
      opts,
    )
    .await
    .map_err(|error| {
      log::warn!("unable to update player balance after join - {}", error);
      Error::from_str(400, "balance-after-join-update")
    })?
    .ok_or_else(|| {
      log::warn!("no player balance updated");
      Error::from_str(400, "balance-after-join-update")
    })?;

  tables
    .find_one_and_replace(crate::db::lookup_for_uuid(&query.id), &ts, None)
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      Error::from_str(422, "failed")
    })?;

  // TODO: do we care if our attempt to enqueue a job fails from the web thread?
  let job = bankah::jobs::TableJob::reindex();
  request.state().queue(&job).await.map(|_| ()).unwrap_or_else(|error| {
    log::warn!("unable to queue indexing job - {}", error);
    ()
  });

  log::info!("player joined table '{}'", ts.id);
  Body::from_json(&ts).map(|body| Response::builder(200).body(body).build())
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

  let tables = request.state().tables();
  let players = request.state().players();

  let name = names::generate().map_err(|error| {
    log::warn!("unable to generate random name - {}", error);
    Error::from_str(500, "name-generation")
  })?;
  let (ts, ps) = sit_player(TableState::with_name(name), player)?;

  // TODO(player-id): when players are initially inserted into the players collection, their `uuid::Uuid` `id` field
  // is being serialized and persisted as a string. Without the explicit `to_string` here, the query attempts to
  // search for a binary match of the `uuid:Uuid`.
  let query = doc! { "id": ps.id.to_string() };
  let updates = doc! { "$set": { "balance": ps.balance } };
  let opts = FindOneAndUpdateOptions::builder()
    .return_document(ReturnDocument::After)
    .build();

  log::info!("creating table for user {:?} ({:?})", ps.id, query);

  players
    .find_one_and_update(query, updates, opts)
    .await
    .map_err(|error| {
      log::warn!("unable to update player balance after create - {}", error);
      Error::from_str(500, "failed balance update on table creation")
    })?
    .ok_or_else(|| {
      log::warn!("no player to update");
      Error::from_str(500, "failed balance update on table creation")
    })?;

  log::debug!("inserting new table {:?}", ts);

  // TODO(mongo-uuid): we're using a `find_one_and_replace` w/ the upsert call here to circumvent the serialization
  // discrepency between insertion and find/replace methods on the mongodb driver collection.
  let tops = FindOneAndReplaceOptions::builder().upsert(true).build();
  let result = tables
    .find_one_and_replace(crate::db::lookup_for_uuid(&ts.id), &ts, Some(tops))
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      Error::from_str(422, "failed")
    });

  // TODO: do we care if our attempt to enqueue a job fails from the web thread?
  let job = bankah::jobs::TableJob::reindex();
  request.state().queue(&job).await.map(|_| ()).unwrap_or_else(|error| {
    log::warn!("unable to queue indexing job - {}", error);
    ()
  });

  result.and_then(|_r| {
    log::info!("new table created - '{}'", ts.id);
    Body::from_json(&ts).map(|body| Response::builder(200).body(body).build())
  })
}

// ## Route
// Leave table.
pub async fn leave(mut request: Request) -> Result {
  let query = request.body_json::<TableActionPayload>().await?;
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "unauth"))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .ok_or(Error::from_str(404, "no-user"))?
    .player()
    .ok_or(Error::from_str(404, "no-player"))?;

  // TODO(table-id): Unlike player ids, it looks like the peristed, serialized bson data for tables uses a binary
  // representation for the `uuid:Uuid` type.
  let search = crate::db::lookup_for_uuid(&query.id);
  log::debug!("user '{}' leaving table '{}' ({:?})", player.id, query.id, search);

  let tables = request.state().tables();
  let players = request.state().players();

  let state = tables
    .find_one(search.clone(), None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      error
    })?
    .ok_or_else(|| {
      log::warn!("unable to find table");
      Error::from_str(404, "table-missing")
    })?;

  let (ts, ps) = stand_player(state, player)?;

  log::debug!("table save, applying player balance - {:?}", ps);

  tables.replace_one(search, &ts, None).await.map_err(|error| {
    log::warn!("unable to persist table updates - {}", error);
    error
  })?;

  players
    .update_one(
      doc! { "id": ps.id.to_string() },
      doc! { "$set": { "balance": ps.balance } },
      None,
    )
    .await
    .map_err(|error| {
      log::warn!("unable to persist new player balance - {}", error);
      error
    })?;

  log::trace!("player '{}' finished leaving", ps.id);

  // TODO: do we care if our attempt to enqueue a job fails from the web thread?
  let job = bankah::jobs::TableJob::reindex();
  request
    .state()
    .queue(&job)
    .await
    .map(|_| {
      log::debug!("successfully queued indexing job");
      ()
    })
    .unwrap_or_else(|error| {
      log::warn!("unable to queue indexing job - {}", error);
      ()
    });

  Body::from_json(&ts).map(|body| Response::builder(200).body(body).build())
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
