use core::iter::FromIterator;

pub mod io;

#[derive(Debug, PartialEq, Clone)]
pub enum Bet {
  PassLine(u32),
  Field(u32),
  Come(u32),
}

#[derive(Clone)]
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
}

#[derive(Debug, Default, Clone)]
pub struct Table {
  bets: Vec<Bet>,
  rolls: Vec<Roll>,
}

impl Table {
  pub fn roll(self) -> Self {
    let mut buffer = [0u8, 2];

    if let Err(error) = getrandom::getrandom(&mut buffer) {
      log::warn!("unable to generate random numbers - {:?}", error);
      return Table { ..self };
    }

    let roll = buffer.iter().map(|item| item.rem_euclid(6) + 1).collect::<Roll>();
    log::debug!("generated roll - {:?}", roll);
    let rolls = self.rolls.into_iter().chain(Some(roll)).collect::<Vec<Roll>>();
    Table { bets: self.bets, rolls }
  }
}

#[derive(Debug, Default)]
pub struct Player {
  balance: u32,
}
