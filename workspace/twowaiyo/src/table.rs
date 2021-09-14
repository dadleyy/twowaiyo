use std::collections::HashMap;
use uuid;

use super::bets::Bet;
use super::constants;
use super::errors::CarryError;
use super::player::Player;
use super::roll::Roll;
use super::seat::Seat;

#[derive(Default, Clone)]
pub struct Table {
  button: Option<u8>,
  seats: HashMap<uuid::Uuid, Seat>,
  rolls: Vec<Roll>,
}

fn apply_bet(mut table: Table, player: &Player, bet: &Bet) -> Result<Table, CarryError<Table>> {
  let seat = table
    .seats
    .remove(&player.id)
    .ok_or_else(|| CarryError::new(table.clone(), "missing seat"))?;

  let updated = seat.bet(bet).unwrap_or_else(|error| {
    log::warn!("unable to make bet - {:?}", error);
    error.consume()
  });

  table.seats.insert(player.id, updated);

  Ok(table)
}

impl Table {
  pub fn bet(self, player: &Player, bet: &Bet) -> Result<Self, CarryError<Self>> {
    let valid = match (self.button, bet) {
      (Some(_), Bet::Pass(_)) => Err(CarryError::new(self, constants::PASS_ON_ERROR)),
      (None, Bet::Place(_, _)) => Err(CarryError::new(self, constants::PLACE_OFF_ERROR)),
      (None, Bet::Come(_)) => Err(CarryError::new(self, constants::COME_OFF_ERROR)),
      (None, Bet::PassOdds(_, _)) => Err(CarryError::new(self, constants::PASS_ODDS_OFF_ERROR)),
      (None, Bet::Hardway(_, _)) => Err(CarryError::new(self, constants::HARDWAY_OFF_ERROR)),
      _ => Ok(self),
    };

    valid.and_then(|table| apply_bet(table, player, bet))
  }

  pub fn sit(self, player: &mut Player) -> Self {
    let Table {
      button,
      mut seats,
      rolls,
    } = self;

    seats.insert(player.id, Seat::with_balance(player.balance));
    player.balance = 0;
    Table { button, seats, rolls }
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

    Table { seats, rolls, button }
  }
}

impl std::fmt::Debug for Table {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    writeln!(formatter, "table")?;
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
