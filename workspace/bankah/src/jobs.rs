use crate::state::BetState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BetJob {
  pub bet: BetState,
  pub player: uuid::Uuid,
  pub table: uuid::Uuid,
  pub version: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RollJob {
  pub table: uuid::Uuid,
  pub version: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobWapper<T> {
  pub job: T,
  pub id: uuid::Uuid,
  pub attempts: u8,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TableJob {
  Bet(JobWapper<BetJob>),
  Roll(JobWapper<RollJob>),
}

impl TableJob {
  pub fn id(&self) -> uuid::Uuid {
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

  pub fn roll(table: uuid::Uuid, version: uuid::Uuid) -> Self {
    let id = uuid::Uuid::new_v4();
    let job = RollJob { table, version };
    TableJob::Roll(JobWapper { job, id, attempts: 0 })
  }

  pub fn bet(state: BetState, player: uuid::Uuid, table: uuid::Uuid, version: uuid::Uuid) -> Self {
    let id = uuid::Uuid::new_v4();
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
