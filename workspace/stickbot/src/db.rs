use std::io::{Error, ErrorKind, Result};

use mongodb;

use super::constants;

pub use mongodb::bson;
pub use mongodb::bson::doc;
pub use mongodb::options::{FindOneAndReplaceOptions, FindOneAndUpdateOptions, ReturnDocument, UpdateModifications};
pub use mongodb::{Client, Collection};

pub fn mongo_error(error: mongodb::error::Error) -> Error {
  Error::new(ErrorKind::Other, format!("{}", error))
}

pub async fn connect(url: String) -> Result<Client> {
  let mut options = mongodb::options::ClientOptions::parse(url).await.map_err(mongo_error)?;
  options.app_name = Some(constants::MONGO_DB_APP_NAME.to_string());
  Client::with_options(options).map_err(mongo_error)
}

pub fn lookup_for_uuid(id: &uuid::Uuid) -> bson::Document {
  doc! { "id": id.to_string() }
}
