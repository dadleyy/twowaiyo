use serde::Serialize;

pub mod account;
pub mod admin;
pub mod auth;
pub mod bets;
pub mod jobs;
pub mod rolls;
pub mod tables;

use crate::web::{Body, Request, Response, Result};

#[derive(Serialize)]
struct Heartbeat {
  time: chrono::DateTime<chrono::offset::Utc>,
  version: String,
}

impl Default for Heartbeat {
  fn default() -> Self {
    let time = chrono::offset::Utc::now();
    Heartbeat {
      time,
      version: "dev".into(),
    }
  }
}

pub async fn heartbeat(request: Request) -> Result {
  let body = Body::from_json(&Heartbeat {
    version: format!("{}", request.state()),
    ..Heartbeat::default()
  })
  .map_err(|error| {
    log::warn!("unable to serialize heartbeat payload - {}", error);
    tide::Error::from_str(500, "bad-serialize")
  })?;
  let state = request.state();
  let status = state.status().await;
  log::debug!("services status - {:?}", status);
  let response = state
    .command(&kramer::Command::Echo::<String, String>("hello-world".into()))
    .await;
  log::debug!("redis echo response - {:?}", response);
  let response = Response::builder(200).body(body).build();
  Ok(response)
}
