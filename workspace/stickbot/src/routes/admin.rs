use serde::Deserialize;

use crate::constants::MONGO_DB_TABLE_COLLECTION_NAME;
use crate::db;
use crate::web::{cookie as get_cookie, Error, Request, Result};

#[derive(Debug, Deserialize)]
struct BalanceQuery {
  amount: u32,
  player: String,
}

pub async fn set_balance(request: Request) -> Result {
  let cook = get_cookie(&request).ok_or(Error::from_str(404, "unauth"))?;
  let player = request
    .state()
    .authority(cook.value())
    .await
    .and_then(|auth| auth.admin())
    .ok_or(Error::from_str(404, ""))?;

  let query = request.query::<BalanceQuery>()?;
  log::info!(
    "admin {} updating player '{}' balance to {}",
    player.id,
    query.player,
    query.amount
  );

  let players = request.state().players();

  players
    .update_one(
      db::doc! { "id": query.player },
      db::doc! { "$set": { "balance": query.amount } },
      None,
    )
    .await
    .map_err(|error| {
      log::warn!("unable to update balance - {}", error);
      Error::from_str(500, "bad-save")
    })?;

  Ok(format!("").into())
}

pub async fn drop_all(request: Request) -> Result {
  let cook = get_cookie(&request).ok_or(Error::from_str(404, "unauth"))?;
  let player = request
    .state()
    .authority(cook.value())
    .await
    .and_then(|auth| auth.admin())
    .ok_or(Error::from_str(404, ""))?;

  log::info!("admin {} dropping all tables", player.id);

  let collection = request.state().tables();

  collection
    .drop(None)
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      Error::from_str(422, "failed")
    })
    .map(|_| {
      log::info!("successfully dropped '{}'", MONGO_DB_TABLE_COLLECTION_NAME);
      format!("").into()
    })
}
