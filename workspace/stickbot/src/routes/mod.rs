use serde::Serialize;

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

pub async fn heartbeat(_req: tide::Request<crate::Services>) -> tide::Result {
  let body = tide::Body::from_json(&Heartbeat::default()).expect("");
  let response = tide::Response::builder(200).body(body).build();
  Ok(response)
}
