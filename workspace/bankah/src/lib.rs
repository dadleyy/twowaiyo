use serde::Serialize;

pub mod jobs;
pub mod state;

#[derive(Debug, Serialize)]
pub struct JobResponse {
  pub job: String,
  pub output: Option<jobs::TableJobOutput>,
}
