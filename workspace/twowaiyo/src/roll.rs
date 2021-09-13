use std::iter::FromIterator;

fn is_place(amount: u8) -> bool {
  match amount {
    4 | 5 | 6 | 8 | 9 | 10 => true,
    _ => false,
  }
}

#[derive(Clone, PartialEq)]
pub struct Roll(u8, u8);

impl FromIterator<u8> for Roll {
  fn from_iter<T: IntoIterator<Item = u8>>(target: T) -> Self {
    let mut iter = target.into_iter();
    let first = iter.next().unwrap_or_default();
    let second = iter.next().unwrap_or_default();
    Roll(first, second)
  }
}

impl std::fmt::Debug for Roll {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "Roll({}, {} | {})", self.0, self.1, self.total())
  }
}

impl Roll {
  pub fn total(&self) -> u8 {
    self.0 + self.1
  }

  pub fn result(&self, button: &Option<u8>) -> RollResult {
    match (button, self.total()) {
      (None, target) if is_place(target) => RollResult::Button(self.total()),
      (Some(target), value) if value == *target => RollResult::Hit,
      (Some(_), 7) => RollResult::Craps,
      (None, 2) | (None, 12) | (None, 3) => RollResult::Craps,

      (Some(_), _) => RollResult::Nothing,
      (None, _) => RollResult::Nothing,
    }
  }
}

#[derive(Debug)]
pub enum RollResult {
  Yo,
  Hit,
  Craps,
  Button(u8),
  Nothing,
}

impl RollResult {
  pub fn button(&self, existing: Option<u8>) -> Option<u8> {
    match self {
      RollResult::Button(value) => Some(*value),
      RollResult::Hit => None,
      RollResult::Nothing => existing,
      RollResult::Craps => None,
      RollResult::Yo => existing,
    }
  }
}
