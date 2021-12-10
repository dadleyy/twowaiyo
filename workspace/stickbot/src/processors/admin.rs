use bankah::jobs::{JobError, TableAdminJob, TableJobOutput};

use crate::db::doc;
use crate::Services;

pub async fn reindex<'a>(services: &Services, job: &TableAdminJob) -> Result<TableJobOutput, JobError> {
  log::info!("attempting to reindex table populations - {:?}", job);
  let tables = services.tables();
  let pipeline = vec![
    doc! { "$project": { "id": 1, "name": 1, "seats": { "$objectToArray": "$seats" } } },
    doc! { "$project": { "id": 1, "name": 1, "population": {
      "$map": {
        "input": "$seats",
        "as": "seat",
        "in": ["$$seat.k", "$$seat.v.nickname"],
      },
    } } },
    doc! { "$merge": { "into": crate::constants::MONGO_DB_TABLE_LIST_COLLECTION_NAME } },
  ];
  tables.aggregate(pipeline, None).await.map_err(|error| {
    log::warn!("unable to perform aggregate - {}", error);
    JobError::Retryable
  })?;
  Ok(TableJobOutput::AdminOk)
}
