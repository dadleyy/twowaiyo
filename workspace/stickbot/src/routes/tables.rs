use crate::constants::MONGO_DB_TABLE_COLLECTION_NAME;
use crate::web::{Body, Error, Request, Response, Result};

use bankah::TableState;
use twowaiyo::Table;

pub async fn drop_all(request: Request) -> Result {
  let collection = request
    .state()
    .collection::<TableState, _>(MONGO_DB_TABLE_COLLECTION_NAME);

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

pub async fn create(request: Request) -> Result {
  let collection = request.state().collection(MONGO_DB_TABLE_COLLECTION_NAME);
  let table = Table::default();

  collection
    .insert_one(TableState::from(&table), None)
    .await
    .map_err(|error| {
      log::warn!("unable to create new table - {:?}", error);
      Error::from_str(422, "failed")
    })
    .and_then(|_r| {
      log::info!("new table created - '{}'", table.identifier());

      let state = TableState {
        id: table.identifier(),
        ..TableState::default()
      };

      Body::from_json(&state).map(|body| Response::builder(200).body(body).build())
    })
}
