use serde::Serialize;

pub mod tables;

use crate::web::{Body, Request, Response, Result};

#[derive(Serialize)]
struct Heartbeat {
  time: chrono::DateTime<chrono::offset::Utc>,
}

impl Default for Heartbeat {
  fn default() -> Self {
    let time = chrono::offset::Utc::now();
    Heartbeat { time }
  }
}

pub async fn heartbeat(_req: Request) -> Result {
  let body = Body::from_json(&Heartbeat::default()).expect("");
  let response = Response::builder(200).body(body).build();
  Ok(response)
}
