use serde::{Deserialize, Serialize};

use crate::constants;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

#[derive(Debug, Deserialize)]
pub struct JobLookupQuery {
  id: String,
}

#[derive(Debug, Serialize)]
pub struct JobLookupResponse {
  id: String,
  output: Option<bankah::TableJobOutput>,
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

  let command = kramer::Command::Hashes(kramer::HashCommand::Get::<_, &str>(
    constants::STICKBOT_BET_RESULTS,
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

      return Body::from_json(&JobLookupResponse {
        id: query.id.clone(),
        output: None,
      })
      .map(|bod| Response::builder(200).body(bod).build());
    }
    other => {
      log::warn!("strange response from job lookup - {:?}", other);
      return Ok(Response::builder(404).build());
    }
  };

  log::debug!("response from result lookup - {:?}", payload);

  let parsed = serde_json::from_str::<bankah::TableJobOutput>(&payload).map_err(|error| {
    log::warn!("unable to parse job output - {}", error);
    Error::from_str(500, "bad-parse")
  })?;

  Body::from_json(&JobLookupResponse {
    id: query.id.clone(),
    output: Some(parsed),
  })
  .map(|bod| Response::builder(200).body(bod).build())
  .map_err(|error| {
    log::warn!("unable to serialize job lookup - {}", error);
    error
  })
}
