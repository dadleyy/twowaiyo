use std::collections::HashMap;
use uuid;

use super::bets::Bet;
use super::errors;
use super::player::Player;
use super::roll::Roll;
use super::rollers::RandomRoller;
use super::seat::{Seat, SeatRuns};

#[derive(Debug, Clone)]
pub struct RunResult<R>
where
  R: Clone + Default + Iterator<Item = u8>,
{
  pub table: Table<R>,
  pub results: HashMap<uuid::Uuid, SeatRuns>,
}

#[derive(Clone)]
pub struct Table<R>
where
  R: Clone + Default + Iterator<Item = u8>,
{
  id: uuid::Uuid,
  roller: Option<uuid::Uuid>,
  button: Option<u8>,
  seats: HashMap<uuid::Uuid, Seat>,
  rolls: Vec<Roll>,
  dice: R,
}

impl Default for Table<RandomRoller> {
  fn default() -> Self {
    let rolls = Vec::with_capacity(crate::constants::MAX_ROLL_HISTORY);
    let id = uuid::Uuid::new_v4();
    let seats = HashMap::with_capacity(100);
    Table {
      id,
      roller: None,
      button: None,
      seats,
      rolls,
      dice: RandomRoller::default(),
    }
  }
}

fn apply_bet<R>(mut table: Table<R>, player: &Player, bet: &Bet) -> Result<Table<R>, errors::CarryError<Table<R>>>
where
  R: Clone + Default + Iterator<Item = u8>,
{
  let seat = table
    .seats
    .remove(&player.id)
    .ok_or_else(|| errors::CarryError::new(table.clone(), errors::RuleViolation::InvalidSeat))?;

  let updated = seat.bet(bet).map_err(|e| e.map(|_| table.clone()))?;

  table.seats.insert(player.id, updated);
  Ok(table)
}

impl<R> Table<R>
where
  R: Clone + Default + Iterator<Item = u8>,
{
  pub fn with_dice(dice: R) -> Self {
    let Table {
      id,
      roller,
      button,
      rolls,
      seats,
      dice: _,
    } = Table::<RandomRoller>::default();

    Table {
      dice,
      button,
      seats,
      roller,
      id,
      rolls,
    }
  }

  pub fn identifier(&self) -> String {
    self.id.to_string()
  }

  pub fn population(&self) -> usize {
    self.seats.len()
  }

  pub fn bet(self, player: &Player, bet: &Bet) -> Result<Self, errors::CarryError<Self>> {
    let valid = match (self.button, bet) {
      (Some(_), Bet::Pass(_)) => Err(errors::CarryError::new(self, errors::PASS_LINE_ALREADY_ON)),
      (None, Bet::Place(_, _)) => Err(errors::CarryError::new(self, errors::PLACE_OFF_ERROR)),
      (None, Bet::Come(_)) => Err(errors::CarryError::new(self, errors::COME_OFF_ERROR)),
      (None, Bet::PassOdds(_, _)) => Err(errors::CarryError::new(self, errors::PASS_ODDS_OFF_ERROR)),
      (None, Bet::Hardway(_, _)) => Err(errors::CarryError::new(self, errors::HARDWAY_OFF_ERROR)),
      _ => Ok(self),
    };

    valid.and_then(|table| apply_bet(table, player, bet))
  }

  pub fn stand(self, player: &mut Player) -> Self {
    let Table {
      id,
      button,
      mut roller,
      dice,
      rolls,
      seats,
    } = self;

    let seats = seats
      .into_iter()
      .filter_map(|(key, value)| {
        if key == player.id {
          let (balance, seat) = value.stand();
          player.balance += balance;
          seat.map(|seat| (key, seat))
        } else {
          Some((key, value))
        }
      })
      .collect::<HashMap<uuid::Uuid, Seat>>();

    roller = roller.and_then(|id| if id == player.id { None } else { Some(id) });

    if let Some((id, _)) = seats.iter().next() {
      roller = roller.or(Some(id.clone()));
    }

    Table {
      id,
      button,
      roller,
      rolls,
      seats,
      dice,
    }
  }

  pub fn sit(self, player: &mut Player) -> Self {
    let Table {
      dice,
      roller,
      id,
      button,
      mut seats,
      rolls,
    } = self;

    let roller = roller.or(Some(player.id.clone()));

    seats.insert(player.id, Seat::with_balance(player.balance));
    player.balance = 0;
    Table {
      id,
      dice,
      button,
      seats,
      roller,
      rolls,
    }
  }

  pub fn roll(mut self) -> RunResult<R> {
    let roll = vec![self.dice.next(), self.dice.next()]
      .into_iter()
      .flatten()
      .collect::<Roll>();

    let result = roll.result(&self.button);
    let button = result.button(self.button);

    log::debug!("generated roll - {:?}, result: {:?}", roll, result);
    let pop = self.population();

    let (seats, results) = self.seats.into_iter().map(|(key, seat)| (key, seat.roll(&roll))).fold(
      (HashMap::with_capacity(pop), HashMap::with_capacity(pop)),
      |(mut seats, mut totals), res| {
        let (uuid, (seat, results)) = res;
        seats.insert(uuid, seat);
        totals.insert(uuid, results);
        (seats, totals)
      },
    );

    let rolls = Some(roll)
      .into_iter()
      .chain(self.rolls.into_iter())
      .take(crate::constants::MAX_ROLL_HISTORY)
      .collect::<Vec<Roll>>();

    let next = Table {
      id: self.id,
      roller: self.roller,
      dice: self.dice,
      button,
      rolls,
      seats,
    };

    RunResult { table: next, results }
  }
}

impl<R> std::fmt::Debug for Table<R>
where
  R: Clone + Default + Iterator<Item = u8>,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    writeln!(formatter, "table {}", self.id)?;
    writeln!(formatter, "button:    {:?}", self.button)?;
    writeln!(formatter, "last roll: {:?}", self.rolls.get(0))?;

    writeln!(formatter, "-- seats:")?;

    for (key, seat) in self.seats.iter() {
      writeln!(formatter, "id:      {}", key)?;
      writeln!(formatter, "{:?}", seat)?;
    }

    Ok(())
  }
}

impl From<&bankah::TableState> for Table<crate::rollers::RandomRoller> {
  fn from(state: &bankah::TableState) -> Self {
    let rolls = state
      .rolls
      .iter()
      .map(|tupe| IntoIterator::into_iter([tupe.0, tupe.1]).collect())
      .collect();

    let seats = state
      .seats
      .iter()
      .map(|(key, state)| (uuid::Uuid::parse_str(&key).unwrap_or_default(), state.into()))
      .collect();

    let roller = state
      .roller
      .as_ref()
      .map(|id| uuid::Uuid::parse_str(&id).unwrap_or_default());

    Table {
      rolls,
      roller,
      seats,
      id: uuid::Uuid::parse_str(&state.id).unwrap_or_default(),
      button: state.button,
      dice: RandomRoller::default(),
    }
  }
}

impl<R> From<&Table<R>> for bankah::TableState
where
  R: Clone + Default + Iterator<Item = u8>,
{
  fn from(table: &Table<R>) -> bankah::TableState {
    let seats = table
      .seats
      .iter()
      .map(|(id, seat)| (id.to_string(), seat.into()))
      .collect();

    bankah::TableState {
      seats,
      id: table.identifier(),
      button: table.button.clone(),
      roller: table.roller.map(|id| id.to_string()),
      rolls: table.rolls.iter().map(|roll| roll.into()).collect(),

      // TODO: the nonce is only represented in the stored data of a table; not the game state/logic itself.
      nonce: String::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::Table;
  use crate::{Bet, Player};

  #[derive(Debug, Default, Clone)]
  struct TestDice(Option<u8>, Option<u8>);

  impl From<(u8, u8)> for TestDice {
    fn from(input: (u8, u8)) -> TestDice {
      TestDice(Some(input.0), Some(input.1))
    }
  }

  impl Iterator for TestDice {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
      self.0.take().or_else(|| self.1.take())
    }
  }

  #[test]
  fn test_run_with_wins() {
    let mut player = Player::default();
    let table = Table::with_dice(TestDice::from((2, 5)))
      .sit(&mut player)
      .bet(&player, &Bet::start_pass(100))
      .unwrap();
    let result = table.roll();
    assert_eq!(result.results.get(&player.id).expect("missing player").losses(), 0);
    assert_eq!(result.results.get(&player.id).expect("missing player").winnings(), 200);
  }

  #[test]
  fn test_run_with_losses() {
    let mut player = Player::default();
    let table = Table::with_dice(TestDice::from((2, 1)))
      .sit(&mut player)
      .bet(&player, &Bet::start_pass(100))
      .unwrap();
    let result = table.roll();
    assert_eq!(result.results.get(&player.id).expect("missing player").losses(), 100);
    assert_eq!(result.results.get(&player.id).expect("missing player").winnings(), 0);
  }

  #[test]
  fn test_roller_default() {
    let table = Table::default();
    assert_eq!(table.roller, None);
  }

  #[test]
  fn test_roller_after_sit() {
    let mut player = Player::default();
    let table = Table::default().sit(&mut player);
    assert_eq!(table.roller, Some(player.id.clone()));
  }

  #[test]
  fn test_roller_after_sit_sit() {
    let mut roller = Player::default();
    let mut player = Player::default();
    let table = Table::default().sit(&mut roller).sit(&mut player);
    assert_eq!(table.roller, Some(roller.id.clone()));
  }

  #[test]
  fn test_roller_after_sit_stand_other() {
    let mut roller = Player::default();
    let mut player = Player::default();
    let table = Table::default().sit(&mut roller).sit(&mut player).stand(&mut player);
    assert_eq!(table.roller, Some(roller.id.clone()));
  }

  #[test]
  fn test_roller_after_sit_sit_stand() {
    let mut roller = Player::default();
    let mut player = Player::default();
    let table = Table::default().sit(&mut roller).sit(&mut player).stand(&mut roller);
    assert_eq!(table.roller, Some(player.id.clone()));
  }

  #[test]
  fn test_roller_after_sit_stand() {
    let mut roller = Player::default();
    let table = Table::default().sit(&mut roller).stand(&mut roller);
    assert_eq!(table.roller, None);
  }
}
