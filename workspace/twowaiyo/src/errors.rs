use super::constants;

#[derive(Debug, PartialEq)]
pub enum PassLineNotEstablishedViolation {
  PassLineOddsBet,
  PlaceBet,
  HardwayBet,
  ComeBet,
}

impl std::fmt::Display for PassLineNotEstablishedViolation {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      PassLineNotEstablishedViolation::PassLineOddsBet => write!(formatter, "{}", constants::PASS_ODDS_OFF_ERROR),
      PassLineNotEstablishedViolation::PlaceBet => write!(formatter, "{}", constants::PLACE_OFF_ERROR),
      PassLineNotEstablishedViolation::HardwayBet => write!(formatter, "{}", constants::HARDWAY_OFF_ERROR),
      PassLineNotEstablishedViolation::ComeBet => write!(formatter, "{}", constants::COME_OFF_ERROR),
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum PassLineEstablishedViolation {
  PassLineBet,
}

impl std::fmt::Display for PassLineEstablishedViolation {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      PassLineEstablishedViolation::PassLineBet => write!(formatter, "{}", constants::PASS_ON_ERROR),
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum PlayerBetViolation {
  MissingComeForOdds,
  MissingPassForOdds,
  InsufficientFunds,
}

impl std::fmt::Display for PlayerBetViolation {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "{:?}", self)
  }
}

#[derive(Debug, PartialEq)]
pub enum RuleViolation {
  PassLineNotEstablished(PassLineNotEstablishedViolation),
  PassLineEstablished(PassLineEstablishedViolation),
  PlayerBetViolation(PlayerBetViolation),
  InvalidSeat,
}

pub const PASS_LINE_ALREADY_ON: RuleViolation =
  RuleViolation::PassLineEstablished(PassLineEstablishedViolation::PassLineBet);
pub const PLACE_OFF_ERROR: RuleViolation =
  RuleViolation::PassLineNotEstablished(PassLineNotEstablishedViolation::PlaceBet);
pub const COME_OFF_ERROR: RuleViolation =
  RuleViolation::PassLineNotEstablished(PassLineNotEstablishedViolation::ComeBet);
pub const HARDWAY_OFF_ERROR: RuleViolation =
  RuleViolation::PassLineNotEstablished(PassLineNotEstablishedViolation::HardwayBet);
pub const PASS_ODDS_OFF_ERROR: RuleViolation =
  RuleViolation::PassLineNotEstablished(PassLineNotEstablishedViolation::PassLineOddsBet);

impl std::fmt::Display for RuleViolation {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      RuleViolation::PassLineNotEstablished(violation) => write!(formatter, "{}", violation),
      RuleViolation::PassLineEstablished(violation) => write!(formatter, "{}", violation),
      RuleViolation::PlayerBetViolation(violation) => write!(formatter, "{}", violation),
      RuleViolation::InvalidSeat => write!(formatter, "missing-seat"),
    }
  }
}

pub struct CarryError<T> {
  kind: T,
  error: RuleViolation,
}

impl<T> CarryError<T> {
  pub fn new(item: T, reason: RuleViolation) -> CarryError<T> {
    CarryError {
      error: reason,
      kind: item,
    }
  }

  pub fn consume(self) -> T {
    self.kind
  }
}

impl<T> std::fmt::Debug for CarryError<T> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "{}", self.error)
  }
}
