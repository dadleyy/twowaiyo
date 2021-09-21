use std::iter::FromIterator;

use super::checks::is_place;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Hardway {
  Four,
  Six,
  Eight,
  Ten,
}

impl From<&u8> for Hardway {
  fn from(way: &u8) -> Hardway {
    match way {
      4 => Hardway::Four,
      6 => Hardway::Six,
      8 => Hardway::Eight,
      10 => Hardway::Ten,
      _ => unreachable!(),
    }
  }
}

impl From<&Hardway> for u8 {
  fn from(way: &Hardway) -> u8 {
    match way {
      Hardway::Four => 4,
      Hardway::Six => 6,
      Hardway::Eight => 8,
      Hardway::Ten => 10,
    }
  }
}

#[derive(Clone, PartialEq)]
pub struct Roll(u8, u8);

impl<U> FromIterator<U> for Roll
where
  U: Into<u8>,
{
  fn from_iter<T: IntoIterator<Item = U>>(target: T) -> Self {
    let mut iter = target.into_iter();
    let first = iter.next().map_or(0, |u| u.into());
    let second = iter.next().map_or(0, |u| u.into());
    Roll(first, second)
  }
}

impl std::fmt::Debug for Roll {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "Roll({}, {} | {})", self.0, self.1, self.total())
  }
}

impl From<&Roll> for (u8, u8) {
  fn from(roll: &Roll) -> (u8, u8) {
    (roll.0, roll.1)
  }
}

impl Roll {
  pub fn total(&self) -> u8 {
    self.0 + self.1
  }

  pub fn left(&self) -> u8 {
    self.0
  }

  pub fn right(&self) -> u8 {
    self.1
  }

  pub fn easyway(&self) -> Option<Hardway> {
    let total = self.total();
    let hardway = self.hardway();

    match (total, hardway) {
      (4, None) => Some(Hardway::Four),
      (6, None) => Some(Hardway::Six),
      (8, None) => Some(Hardway::Eight),
      (10, None) => Some(Hardway::Ten),
      _ => None,
    }
  }

  pub fn hardway(&self) -> Option<Hardway> {
    match (self.0, self.1) {
      (2, 2) => Some(Hardway::Four),
      (3, 3) => Some(Hardway::Six),
      (4, 4) => Some(Hardway::Eight),
      (5, 5) => Some(Hardway::Ten),
      _ => None,
    }
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

#[cfg(test)]
mod test {
  use super::{Hardway, Roll};

  #[test]
  fn easyway_four() {
    let roll = vec![1u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Four));

    let roll = vec![3u8, 1u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Four));

    let roll = vec![2u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), None);
    let roll = vec![1u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), None);
  }

  #[test]
  fn easyway_six() {
    let roll = vec![1u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Six));
    let roll = vec![2u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Six));
    let roll = vec![4u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Six));
    let roll = vec![2u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Six));
    let roll = vec![3u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), None);
  }

  #[test]
  fn easyway_eight() {
    let roll = vec![2u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Eight));

    let roll = vec![3u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Eight));

    let roll = vec![6u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Eight));

    let roll = vec![5u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Eight));

    let roll = vec![4u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), None);
  }

  #[test]
  fn easyway_ten() {
    let roll = vec![4u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Ten));

    let roll = vec![6u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), Some(Hardway::Ten));

    let roll = vec![5u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(roll.easyway(), None);
  }

  #[test]
  fn hardway_four() {
    let roll = vec![1u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![3u8, 1u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![2u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), Some(Hardway::Four));

    let roll = vec![1u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);
  }

  #[test]
  fn hardway_six() {
    let roll = vec![1u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);
    let roll = vec![2u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);
    let roll = vec![4u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);
    let roll = vec![2u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);
    let roll = vec![3u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), Some(Hardway::Six));
  }

  #[test]
  fn hardway_eight() {
    let roll = vec![2u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![3u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![6u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![5u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![4u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), Some(Hardway::Eight));
  }

  #[test]
  fn hardway_ten() {
    let roll = vec![4u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![6u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), None);

    let roll = vec![5u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(roll.hardway(), Some(Hardway::Ten));
  }
}
