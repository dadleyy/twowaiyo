use crate::constants;

/* TODO: this is a way to share the runtime computation of environment specific storage names. The alternative is to
 * compute this once during the initialization of `Services`.
 *
 */

#[derive(Debug, Clone)]
pub enum JobStore {
  Queue,
  Results,
}

impl std::fmt::Display for JobStore {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    let v = match &self {
      JobStore::Queue => std::env::var("STICKBOT_JOB_QUEUE").unwrap_or(constants::STICKBOT_JOB_QUEUE.to_string()),
      JobStore::Results => std::env::var("STICKBOT_JOB_RESULTS").unwrap_or(constants::STICKBOT_JOB_RESULTS.to_string()),
    };

    write!(formatter, "{}", v)
  }
}

#[derive(Debug, Clone)]
pub enum Collection {
  TableList,
  Tables,
  Players,
}

impl std::fmt::Display for Collection {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    let v = match &self {
      Collection::Players => {
        std::env::var("STICKBOT_PLAYER_COLLECTION").unwrap_or(constants::MONGO_DB_PLAYER_COLLECTION_NAME.to_string())
      }
      Collection::Tables => {
        std::env::var("STICKBOT_TABLE_COLLECTION").unwrap_or(constants::MONGO_DB_TABLE_COLLECTION_NAME.to_string())
      }
      Collection::TableList => std::env::var("STICKBOT_TABLE_LIST_COLLECTION")
        .unwrap_or(constants::MONGO_DB_TABLE_LIST_COLLECTION_NAME.to_string()),
    };

    write!(formatter, "{}", v)
  }
}
