use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub enum TargetKind {
  ComeOdds,
  PassOdds,
  Place,
  Hardway,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum RaceType {
  Pass,
  Come,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BetState {
  Race(RaceType, u32, Option<u8>),
  Target(TargetKind, u32, u8),
  Field(u32),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SeatState {
  pub balance: u32,
  pub bets: Vec<BetState>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TableState {
  pub id: String,
  pub button: Option<u8>,
  pub seats: HashMap<String, SeatState>,
  pub rolls: Vec<(u8, u8)>,
  pub nonce: String,
}

impl Default for TableState {
  fn default() -> Self {
    TableState {
      id: String::new(),
      button: None,
      rolls: vec![],
      seats: HashMap::new(),
      nonce: uuid::Uuid::new_v4().to_string(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Player {
  pub id: String,
  pub oid: String,
  pub nickname: String,
  pub balance: u32,
}
