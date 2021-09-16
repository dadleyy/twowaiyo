use uuid;

#[derive(Debug, Clone, Eq)]
pub struct Player {
  pub id: uuid::Uuid,
  pub balance: u32,
}

impl From<&bankah::PlayerState> for Player {
  fn from(state: &bankah::PlayerState) -> Self {
    Player {
      id: uuid::Uuid::parse_str(&state.id).unwrap_or_default(),
      balance: state.balance,
    }
  }
}

impl PartialEq for Player {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
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
