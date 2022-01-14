use crate::state::BetState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct BetJob {
  pub bet: BetState,
  pub player: uuid::Uuid,
  pub table: uuid::Uuid,
  pub version: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RollJob {
  pub table: uuid::Uuid,
  pub version: uuid::Uuid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct JobWapper<T> {
  pub job: T,
  pub id: uuid::Uuid,
  pub attempts: u8,
}

impl<T> JobWapper<T> {
  pub fn wrap(job: T) -> Self {
    let id = uuid::Uuid::new_v4();
    let attempts = 0u8;
    Self { job, attempts, id }
  }
}

impl<T> JobWapper<T> {
  pub fn retry(self) -> Self {
    let JobWapper { job, id, attempts } = self;
    JobWapper {
      job,
      id,
      attempts: attempts + 1,
    }
  }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TableAdminJob {
  ReindexPopulations,
  CleanupPlayerData(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TableJob {
  Bet(JobWapper<BetJob>),
  Roll(JobWapper<RollJob>),
  Sit(JobWapper<(String, String)>),
  Create(JobWapper<String>),
  Stand(JobWapper<(String, String)>),
  Admin(JobWapper<TableAdminJob>),
}

impl TableJob {
  pub fn id(&self) -> uuid::Uuid {
    match self {
      TableJob::Bet(inner) => inner.id.clone(),
      TableJob::Roll(inner) => inner.id.clone(),
      TableJob::Sit(inner) => inner.id.clone(),
      TableJob::Create(inner) => inner.id.clone(),
      TableJob::Stand(inner) => inner.id.clone(),
      TableJob::Admin(inner) => inner.id.clone(),
    }
  }

  pub fn sit(table: String, player: String) -> Self {
    TableJob::Sit(JobWapper::wrap((table, player)))
  }

  pub fn stand(table: String, player: String) -> Self {
    TableJob::Stand(JobWapper::wrap((table, player)))
  }

  pub fn admin(job: TableAdminJob) -> Self {
    let id = uuid::Uuid::new_v4();
    TableJob::Admin(JobWapper { job, id, attempts: 0 })
  }

  pub fn retry(self) -> Option<Self> {
    match self {
      TableJob::Bet(inner) => Some(TableJob::Bet(inner.retry())),
      _ => None,
    }
  }

  pub fn reindex() -> Self {
    let id = uuid::Uuid::new_v4();
    TableJob::Admin(JobWapper {
      job: TableAdminJob::ReindexPopulations,
      id,
      attempts: 0,
    })
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
#[serde(rename_all = "snake_case")]
pub enum BetFailureReason {
  InsufficientFunds,
  InvalidComeBet,
  MissingComeForOdds,
  MissingPassForOdds,
  Other,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableJobOutput {
  BetProcessed,
  BetStale,
  BetFailed(BetFailureReason),
  RollProcessed,
  RollStale,
  AdminOk,
  StandOk,
  FinalStandOk,
  SitOk,
  TableCreated(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct JobResult<T> {
  completed: Option<chrono::DateTime<chrono::Utc>>,
  output: Option<T>,
  id: uuid::Uuid,
}

impl<T> JobResult<T> {
  pub fn empty(id: uuid::Uuid) -> Self {
    return Self {
      id,
      completed: None,
      output: None,
    };
  }

  pub fn wrap(id: uuid::Uuid, inner: T) -> Self {
    let completed = Some(chrono::Utc::now());
    Self {
      output: Some(inner),
      id,
      completed,
    }
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobError {
  Retryable,
  Terminal(String),
}
