use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TargetKind {
  ComeOdds,
  PassOdds,
  Place,
  Hardway,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum RaceType {
  Pass,
  Come,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BetState {
  Race(RaceType, u32, Option<u8>),
  Target(TargetKind, u32, u8),
  Field(u32),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SeatState {
  pub balance: u32,
  pub nickname: String,
  pub history: Vec<(BetState, bool, u32)>,
  pub seated_at: chrono::DateTime<chrono::Utc>,
  pub bets: Vec<BetState>,
}

impl Default for SeatState {
  fn default() -> Self {
    SeatState {
      balance: 0,
      nickname: String::default(),
      history: Vec::with_capacity(0),
      seated_at: chrono::Utc::now(),
      bets: Vec::with_capacity(0),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TableIndexState {
  pub id: uuid::Uuid,
  pub name: String,
  // pub population: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TableState {
  pub id: uuid::Uuid,
  pub name: String,
  pub button: Option<u8>,
  pub roller: Option<uuid::Uuid>,
  pub seats: HashMap<uuid::Uuid, SeatState>,
  pub rolls: Vec<(u8, u8)>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub nonce: uuid::Uuid,
}

impl TableState {
  pub fn with_name<S>(name: S) -> Self
  where
    S: std::fmt::Display,
  {
    Self {
      name: format!("{}", name),
      created_at: chrono::Utc::now(),
      ..Self::default()
    }
  }
}

impl Default for TableState {
  fn default() -> Self {
    TableState {
      id: uuid::Uuid::new_v4(),
      name: String::default(),
      created_at: chrono::Utc::now(),
      roller: None,
      button: None,
      rolls: vec![],
      seats: HashMap::new(),
      nonce: uuid::Uuid::new_v4(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerState {
  pub id: uuid::Uuid,
  pub oid: String,
  pub emails: Vec<String>,
  pub nickname: String,
  pub balance: u32,
}
