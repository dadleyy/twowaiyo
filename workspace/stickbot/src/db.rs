use std::io::{Error, ErrorKind, Result};

use mongodb;

use super::constants;

pub use mongodb::bson::doc;

pub fn mongo_error(error: mongodb::error::Error) -> Error {
  Error::new(ErrorKind::Other, format!("{}", error))
}

pub async fn connect(url: String) -> Result<mongodb::Client> {
  let mut options = mongodb::options::ClientOptions::parse(url).await.map_err(mongo_error)?;
  options.app_name = Some(constants::MONGO_DB_APP_NAME.to_string());
  mongodb::Client::with_options(options).map_err(mongo_error)
}
