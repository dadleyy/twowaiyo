use async_std::stream::StreamExt;
use serde::Deserialize;

use crate::constants::{MONGO_DB_PLAYER_COLLECTION_NAME, MONGO_DB_TABLE_COLLECTION_NAME};
use crate::db::{doc, FindOneAndUpdateOptions, ReturnDocument};
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

use bankah::TableState;
use twowaiyo::Table;

// List tables.
pub async fn list(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  log::info!("listing tables for '{:?}'", player);
  let collection = request
    .state()
    .collection::<TableState, _>(MONGO_DB_TABLE_COLLECTION_NAME);

  let mut tables = collection.find(None, None).await.map_err(|error| {
    log::warn!("unable to query tables - {}", error);
    Error::from_str(500, "load-tables")
  })?;

  let mut page: Vec<bankah::TableState> = Vec::with_capacity(10);

  while let Some(doc) = tables.next().await {
    log::info!("found doc - {:?}", doc);

    if let Ok(state) = doc {
      page.push(state)
    }
  }

  let body = Body::from_json(&page)?;
  Ok(Response::builder(200).body(body).build())
}

#[derive(Debug, Deserialize)]
struct TableActionPayload {
  pub id: String,
}

// Joins a table.
pub async fn join(mut request: Request) -> Result {
  let query = request.body_json::<TableActionPayload>().await.map_err(|error| {
    log::warn!("unable to parse join-table payload - {}", error);
    error
  })?;

  let cookie = get_cookie(&request).ok_or(Error::from_str(404, ""))?;
  let mut player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|auth| auth.player())
    .map(|player| twowaiyo::Player::from(&player))
    .ok_or(Error::from_str(404, "no-player"))?;

  if player.balance == 0 {
    return Ok(
      Response::builder(422)
        .body(Body::from_string("no-balance".into()))
        .build(),
    );
  }

  let tables = request
    .state()
    .collection::<bankah::TableState, _>(MONGO_DB_TABLE_COLLECTION_NAME);
  let players = request
    .state()
    .collection::<bankah::TableState, _>(MONGO_DB_PLAYER_COLLECTION_NAME);

  let state = tables
    .find_one(doc! { "id": query.id }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      Error::from_str(500, "lookup")
    })?
    .ok_or(Error::from_str(404, "no-table"))?;

  // Apply business logic
  let table = twowaiyo::Table::from(&state);
  let table = table.sit(&mut player);

  let opts = FindOneAndUpdateOptions::builder()
    .return_document(ReturnDocument::After)
    .build();

  match players
    .find_one_and_update(
      doc! { "id": player.id.to_string() },
      doc! { "$set": { "balance": player.balance } },
      opts,
    )
    .await
  {
    Err(error) => log::warn!("unable to update player balance after join - {}", error),
    Ok(None) => log::warn!("no player to update"),
    Ok(Some(_)) => log::info!("player balance updated"),
  }

  // Make sure to keep our `nonce` the same - joining doesn't affect version.
  let mut replacement = bankah::TableState::from(&table);
  replacement.nonce = state.nonce;

  tables
    .replace_one(doc! { "id": table.identifier() }, &replacement, None)
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      Error::from_str(422, "failed")
    })
    .and_then(|_r| {
      log::info!("player joined table '{}'", table.identifier());
      Body::from_json(&replacement).map(|body| Response::builder(200).body(body).build())
    })
}

// Creates a new table and sits the player.
pub async fn create(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, ""))?;
  let mut player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|auth| auth.player())
    .map(|player| twowaiyo::Player::from(&player))
    .ok_or(Error::from_str(404, "no-player"))?;

  log::info!("creating table for user {:?}", player.id);

  let tables = request.state().collection(MONGO_DB_TABLE_COLLECTION_NAME);
  let players = request
    .state()
    .collection::<bankah::PlayerState, _>(MONGO_DB_PLAYER_COLLECTION_NAME);
  let table = Table::default().sit(&mut player);

  let opts = FindOneAndUpdateOptions::builder()
    .return_document(ReturnDocument::After)
    .build();

  match players
    .find_one_and_update(
      doc! { "id": player.id.to_string() },
      doc! { "$set": { "balance": player.balance } },
      opts,
    )
    .await
  {
    Err(error) => log::warn!("unable to update player balance after create - {}", error),
    Ok(None) => log::warn!("no player to update"),
    Ok(Some(_)) => log::info!("player balance updated"),
  }

  tables
    .insert_one(TableState::from(&table), None)
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      Error::from_str(422, "failed")
    })
    .and_then(|_r| {
      log::info!("new table created - '{}'", table.identifier());
      let state: TableState = (&table).into();
      Body::from_json(&state).map(|body| Response::builder(200).body(body).build())
    })
}

// Leave table.
pub async fn leave(mut request: Request) -> Result {
  let query = request.body_json::<TableActionPayload>().await.map_err(|error| {
    log::warn!("unable to parse leave payload - {}", error);
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

  log::info!("user '{}' attempting to leave table '{}'", player.id, query.id);

  let tables = request
    .state()
    .collection::<bankah::TableState, _>(MONGO_DB_TABLE_COLLECTION_NAME);
  let players = request
    .state()
    .collection::<bankah::PlayerState, _>(MONGO_DB_PLAYER_COLLECTION_NAME);

  let state = tables
    .find_one(doc! { "id": query.id.as_str() }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to find table - {}", error);
      error
    })?
    .ok_or(Error::from_str(404, "table-missing"))?;

  let table = twowaiyo::Table::from(&state);
  let mut seated = twowaiyo::Player::from(&player);
  let table = table.stand(&mut seated);

  log::debug!("{:?} (population {})", table, table.population());

  // Do not update nonce, leaving doesnt change state.
  let mut updated = bankah::TableState::from(&table);
  updated.nonce = state.nonce;

  tables
    .replace_one(doc! { "id": query.id.as_str() }, &updated, None)
    .await
    .map_err(|error| {
      log::warn!("unable to persist table updates - {}", error);
      error
    })?;

  log::debug!("table save, applying player balance - {:?}", player);

  players
    .update_one(
      doc! { "id": player.id.as_str() },
      doc! { "$set": { "balance": seated.balance } },
      None,
    )
    .await
    .map_err(|error| {
      log::warn!("unable to persist new player balance - {}", error);
      error
    })?;

  log::info!("finished leaving");

  Body::from_json(&updated).map(|body| Response::builder(200).body(body).build())
}
