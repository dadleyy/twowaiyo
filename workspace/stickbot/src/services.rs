use std::io::Result;

use crate::auth;
use crate::constants;
use crate::db;

#[derive(Clone)]
pub struct Services {
  db: db::Client,
}

impl Services {
  pub fn collection<T, N>(&self, name: N) -> db::Collection<T>
  where
    N: AsRef<str>,
  {
    let db = self.db.database(constants::MONGO_DB_DATABASE_NAME);
    db.collection::<T>(name.as_ref())
  }

  pub async fn authority<T>(&self, token: T) -> Option<auth::Authority>
  where
    T: std::fmt::Display,
  {
    let claims = auth::Claims::decode(&token).ok()?;
    let collection = self.collection::<bankah::PlayerState, _>(constants::MONGO_DB_PLAYER_COLLECTION_NAME);
    log::info!("decoded claims '{:?}', fetching user", claims);
    collection
      .find_one(db::doc! { "oid": claims.oid.clone(), "id": claims.id.clone() }, None)
      .await
      .ok()
      .and_then(|maybe_player| maybe_player.map(|player| auth::Authority::Player(player)))
  }

  pub async fn new() -> Result<Self> {
    let url = std::env::var(constants::MONGO_DB_ENV_URL).unwrap_or_default();
    log::debug!("attempting to establish mongo connection at {}", url);
    let mongo = db::connect(url).await?;
    Ok(Services { db: mongo })
  }
}
