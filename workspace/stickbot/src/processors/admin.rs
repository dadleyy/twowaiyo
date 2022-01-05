use async_std::stream::StreamExt;

use bankah::jobs::{JobError, TableAdminJob, TableJob, TableJobOutput};

use crate::db::{doc, FindOneAndReplaceOptions, ReturnDocument};
use crate::Services;

pub async fn reindex(services: &Services, job: &TableAdminJob) -> Result<TableJobOutput, JobError> {
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

pub async fn cleanup(services: &Services, id: &String) -> Result<TableJobOutput, JobError> {
  log::debug!("cleaning up player '{}'", id);

  let mut tables = match services
    .tables()
    .find(doc! { format!("seats.{}", id): { "$exists": true } }, None)
    .await
  {
    Err(error) => {
      log::warn!("unable to find any tables for user - {}", error);
      return Ok(TableJobOutput::AdminOk);
    }
    Ok(cursor) => cursor,
  };

  while let Some(doc) = tables.next().await {
    let mut state = match doc {
      Err(error) => {
        log::warn!("error loading next able - {}", error);
        continue;
      }
      Ok(table) => table,
    };

    state.seats = state
      .seats
      .into_iter()
      .filter(|(seat, _)| &seat.to_string() != id)
      .collect();

    let opts = FindOneAndReplaceOptions::builder()
      .return_document(ReturnDocument::After)
      .build();

    if let Err(error) = services
      .tables()
      .find_one_and_replace(crate::db::lookup_for_uuid(&state.id), &state, opts)
      .await
    {
      log::warn!("failed cleanup '{}' on table '{}': {}", id, &state.id, error);
    }

    log::debug!("next table - {:?}", state);
  }

  log::debug!("cleanup for player '{}' complete, reindexing tables", id);

  if let Err(error) = services.queue(&TableJob::reindex()).await {
    log::warn!("unable to queue reindexing job - {}", error);
  }

  Ok(TableJobOutput::AdminOk)
}
