use std::collections::HashMap;
use uuid;

use super::bets::Bet;
use super::errors;
use super::player::Player;
use super::roll::Roll;
use super::seat::Seat;

#[derive(Debug, Clone)]
pub struct RunResult {
  table: Table,
  results: HashMap<uuid::Uuid, Bet>,
}

#[derive(Clone)]
pub struct Table {
  id: uuid::Uuid,
  roller: Option<uuid::Uuid>,
  button: Option<u8>,
  seats: HashMap<uuid::Uuid, Seat>,
  rolls: Vec<Roll>,
}

impl From<&bankah::TableState> for Table {
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
    }
  }
}

impl From<&Table> for bankah::TableState {
  fn from(table: &Table) -> bankah::TableState {
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

impl Default for Table {
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
    }
  }
}

fn apply_bet(mut table: Table, player: &Player, bet: &Bet) -> Result<Table, errors::CarryError<Table>> {
  let seat = table
    .seats
    .remove(&player.id)
    .ok_or_else(|| errors::CarryError::new(table.clone(), errors::RuleViolation::InvalidSeat))?;

  let updated = seat.bet(bet).map_err(|e| e.map(|_| table.clone()))?;

  table.seats.insert(player.id, updated);
  Ok(table)
}

impl Table {
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
    }
  }

  pub fn sit(self, player: &mut Player) -> Self {
    let Table {
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
      button,
      seats,
      roller,
      rolls,
    }
  }

  pub fn run(self) -> RunResult {
    RunResult {
      table: self,
      results: HashMap::new(),
    }
  }

  pub fn roll(self) -> Self {
    let mut buffer = [0u8, 2];

    if let Err(error) = getrandom::getrandom(&mut buffer) {
      log::warn!("unable to generate random numbers - {:?}", error);
      return Table { ..self };
    }

    let roll = buffer.iter().map(|item| item.rem_euclid(6) + 1).collect::<Roll>();

    let result = roll.result(&self.button);
    let button = result.button(self.button);

    log::debug!("generated roll - {:?}, result: {:?}", roll, result);

    let seats = self.seats.into_iter().map(|(k, v)| (k, v.roll(&roll))).collect();

    let rolls = Some(roll)
      .into_iter()
      .chain(self.rolls.into_iter())
      .take(crate::constants::MAX_ROLL_HISTORY)
      .collect::<Vec<Roll>>();

    Table {
      id: self.id,
      roller: self.roller,
      seats,
      rolls,
      button,
    }
  }
}

impl std::fmt::Debug for Table {
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

#[cfg(test)]
mod tests {
  use super::Table;
  use crate::Player;

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
