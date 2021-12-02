use serde::Deserialize;

use crate::constants::MONGO_DB_TABLE_COLLECTION_NAME;
use crate::db;
use crate::web::{cookie as get_cookie, Error, Request, Result};

#[derive(Debug, Deserialize)]
struct BalanceQuery {
  amount: u32,
}

pub async fn set_balance(request: Request) -> Result {
  let cook = get_cookie(&request).ok_or(Error::from_str(404, "unauth"))?;
  let player = request
    .state()
    .authority(cook.value())
    .await
    .and_then(|auth| auth.player())
    .ok_or(Error::from_str(404, ""))?;
  let query = request.query::<BalanceQuery>()?;
  log::info!("updating player '{}' balance to {}", player.id, query.amount);

  let players = request.state().players();

  players
    .update_one(
      db::doc! { "id": player.id.to_string() },
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
