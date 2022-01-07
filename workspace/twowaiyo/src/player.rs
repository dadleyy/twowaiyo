use uuid;

use bankah::state::PlayerState;

#[derive(Debug, Clone, Eq)]
pub struct Player {
  pub id: uuid::Uuid,
  pub balance: u32,
}

impl From<&Player> for PlayerState {
  fn from(state: &Player) -> PlayerState {
    PlayerState {
      id: state.id.clone(),
      balance: state.balance,
      emails: vec![],
      tables: vec![],
      nickname: String::default(),
      oid: String::default(),
    }
  }
}

impl From<&PlayerState> for Player {
  fn from(state: &PlayerState) -> Self {
    Player {
      id: state.id.clone(),
      balance: state.balance,
    }
  }
}

impl PartialEq for Player {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Player {
  pub fn with_balance(balance: u32) -> Self {
    Self {
      balance,
      ..Self::default()
    }
  }
}

impl Default for Player {
  fn default() -> Self {
    Player {
      id: uuid::Uuid::new_v4(),
      balance: 10000,
    }
  }
}
