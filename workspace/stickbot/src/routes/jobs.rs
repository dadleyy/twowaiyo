use serde::Deserialize;

use bankah::jobs::{JobResult, TableJobOutput};

use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

#[derive(Debug, Deserialize)]
pub struct JobLookupQuery {
  id: String,
}

pub async fn find(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let query = request.query::<JobLookupQuery>()?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  log::debug!("player '{}' checking on job '{}'", player.id, query.id);
  let storage = format!("{}", crate::env::JobStore::Results);

  let command = kramer::Command::Hashes(kramer::HashCommand::Get::<_, &str>(
    storage.as_str(),
    Some(kramer::Arity::One(query.id.as_str())),
  ));

  let response = request.state().command(&command).await.map_err(|error| {
    log::warn!("unable to fetch requested job - {}", error);
    Error::from_str(500, "bad-lookup")
  })?;

  let payload = match &response {
    kramer::Response::Item(kramer::ResponseValue::String(inner)) => inner.clone(),
    kramer::Response::Item(kramer::ResponseValue::Empty) => {
      log::debug!("nothing in job result store for '{}' yet", query.id);
      return Body::from_json(&JobResult::empty(query.id) as &JobResult<u8>)
        .map(|bod| Response::builder(200).body(bod).build());
    }
    other => {
      log::warn!("strange response from job lookup - {:?}", other);
      return Ok(Response::builder(404).build());
    }
  };

  log::debug!("response from result lookup - {:?}", payload);

  let parsed = serde_json::from_str::<JobResult<TableJobOutput>>(&payload).map_err(|error| {
    log::warn!("unable to parse job output - {}", error);
    Error::from_str(500, "bad-parse")
  })?;

  Body::from_json(&parsed)
    .map(|bod| Response::builder(200).body(bod).build())
    .map_err(|error| {
      log::warn!("unable to serialize job lookup - {}", error);
      error
    })
}
