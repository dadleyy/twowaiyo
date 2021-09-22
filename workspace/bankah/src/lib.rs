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

#[derive(Debug, Deserialize, Serialize)]
pub struct SeatState {
  pub balance: u32,
  pub bets: Vec<BetState>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TableState {
  pub id: String,
  pub button: Option<u8>,
  pub roller: Option<String>,
  pub seats: HashMap<String, SeatState>,
  pub rolls: Vec<(u8, u8)>,
  pub nonce: String,
}

impl Default for TableState {
  fn default() -> Self {
    TableState {
      id: String::new(),
      roller: None,
      button: None,
      rolls: vec![],
      seats: HashMap::new(),
      nonce: uuid::Uuid::new_v4().to_string(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerState {
  pub id: String,
  pub oid: String,
  pub nickname: String,
  pub balance: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BetJob {
  pub bet: BetState,
  pub player: String,
  pub table: String,
  pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RollJob {
  pub table: String,
  pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobWapper<T> {
  pub job: T,
  pub id: String,
  pub attempts: u8,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TableJob {
  Bet(JobWapper<BetJob>),
  Roll(JobWapper<RollJob>),
}

impl TableJob {
  pub fn id(&self) -> String {
    match self {
      TableJob::Bet(inner) => inner.id.clone(),
      TableJob::Roll(inner) => inner.id.clone(),
    }
  }

  pub fn retry(&self) -> Option<Self> {
    match self {
      TableJob::Bet(inner) => Some(TableJob::Bet(JobWapper {
        attempts: inner.attempts + 1,
        ..inner.clone()
      })),
      _ => None,
    }
  }

  pub fn roll(table: String, version: String) -> Self {
    let id = uuid::Uuid::new_v4().to_string();
    let job = RollJob { table, version };
    TableJob::Roll(JobWapper { job, id, attempts: 0 })
  }

  pub fn bet(state: BetState, player: String, table: String, version: String) -> Self {
    let id = uuid::Uuid::new_v4().to_string();
    let job = BetJob {
      bet: state,
      player,
      table,
      version,
    };
    TableJob::Bet(JobWapper { job, id, attempts: 0 })
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BetFailureReason {
  InsufficientFunds,
  InvalidComeBet,
  MissingComeForOdds,
  MissingPassForOdds,
  Other,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TableJobOutput {
  BetProcessed,
  BetStale,
  BetFailed(BetFailureReason),
  RollProcessed,
  RollStale,
}

#[derive(Debug, Serialize)]
pub enum JobError {
  Retryable,
  Terminal(String),
}
