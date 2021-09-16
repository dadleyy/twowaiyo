use std::collections::HashMap;
use uuid;

use super::bets::Bet;
use super::errors;
use super::player::Player;
use super::roll::Roll;
use super::seat::Seat;

#[derive(Clone)]
pub struct Table {
  id: uuid::Uuid,
  button: Option<u8>,
  seats: HashMap<uuid::Uuid, Seat>,
  rolls: Vec<Roll>,
}

impl From<bankah::TableState> for Table {
  fn from(state: bankah::TableState) -> Self {
    let rolls = state
      .rolls
      .into_iter()
      .map(|tupe| IntoIterator::into_iter([tupe.0, tupe.1]).collect())
      .collect();

    let seats = state
      .seats
      .iter()
      .map(|(key, state)| (uuid::Uuid::parse_str(&key).unwrap_or_default(), state.into()))
      .collect();

    Table {
      rolls,
      seats,
      id: uuid::Uuid::parse_str(&state.id).unwrap_or_default(),
      button: state.button,

      ..Table::default()
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
      ..bankah::TableState::default()
    }
  }
}

impl Default for Table {
  fn default() -> Self {
    let rolls = Vec::with_capacity(2);
    let id = uuid::Uuid::new_v4();
    let seats = HashMap::with_capacity(100);
    Table {
      id,
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

  let updated = seat.bet(bet).unwrap_or_else(|error| {
    log::warn!("unable to make bet - {:?}", error);
    error.consume()
  });

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
      .collect();

    Table {
      id,
      button,
      rolls,
      seats,
    }
  }

  pub fn sit(self, player: &mut Player) -> Self {
    let Table {
      id,
      button,
      mut seats,
      rolls,
    } = self;

    seats.insert(player.id, Seat::with_balance(player.balance));
    player.balance = 0;
    Table {
      id,
      button,
      seats,
      rolls,
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
      .take(2)
      .collect::<Vec<Roll>>();

    Table {
      id: self.id,
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
