use std::collections::HashMap;

use uuid;

pub mod bets;
pub mod errors;
pub mod io;
pub mod roll;

use bets::Bet;
use roll::Roll;

use errors::CarryError;

#[derive(Default, Clone)]
pub struct Dealer {
  player: Player,
  table: Table,
}

impl std::fmt::Debug for Dealer {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "{:?}", self.table)
  }
}

impl Dealer {
  pub fn new(table: Table, player: Player) -> Self {
    let table = table.sit(&player);
    Dealer { table, player }
  }

  pub fn roll(self) -> Self {
    let Dealer { table, player } = self;
    let new_table = table.roll();
    Dealer {
      table: new_table,
      player,
    }
  }

  pub fn bet(self, bet: &Bet) -> Result<Self, CarryError<Self>> {
    let Dealer { player, table } = self;

    table
      .bet(&player, bet)
      .map_err(|error| {
        log::warn!("unable to apply bet, should skip");
        let reason = format!("{:?}", error);
        let dealer = Dealer {
          table: error.consume(),
          player: player.clone(),
        };
        CarryError::new(dealer, reason.as_str())
      })
      .and_then(|table| Ok(Dealer { table, player }))
  }
}

#[derive(Debug, Clone, Default)]
pub struct Seat {
  bets: Vec<Bet>,
  balance: u32,
}

impl Seat {
  pub fn with_balance(balance: u32) -> Self {
    Seat {
      balance,
      ..Self::default()
    }
  }
}

#[derive(Default, Clone)]
pub struct Table {
  button: Option<u8>,
  seats: HashMap<uuid::Uuid, Seat>,
  rolls: Vec<Roll>,
}

fn apply_bet(mut table: Table, player: &Player, bet: &Bet) -> Result<Table, CarryError<Table>> {
  let seat = table
    .seats
    .get(&player.id)
    .ok_or_else(|| CarryError::new(table.clone(), "missing seat"))?;

  if seat.balance < bet.weight() {
    return Err(CarryError::new(table, "insufficient funds"));
  }

  let bets = seat
    .bets
    .iter()
    .chain(Some(bet))
    .map(|b| b.clone())
    .collect::<Vec<Bet>>();

  let updated = Seat {
    bets,
    balance: seat.balance - bet.weight(),
  };

  table.seats.insert(player.id, updated);

  Ok(table)
}

impl std::fmt::Debug for Table {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    writeln!(formatter, "table")?;
    writeln!(formatter, "button:    {:?}", self.button)?;
    writeln!(formatter, "last roll: {:?}", self.rolls.get(0))?;

    writeln!(formatter, "-- seats:")?;
    for (_, seat) in self.seats.iter() {
      writeln!(formatter, "  balance: {}", seat.balance)?;
      writeln!(formatter, "  -- bets:")?;
      for bet in seat.bets.iter() {
        writeln!(formatter, "  {:?}", bet)?;
      }
    }

    Ok(())
  }
}

impl Table {
  pub fn bet(self, player: &Player, bet: &Bet) -> Result<Self, CarryError<Self>> {
    let valid = match (self.button, bet) {
      (Some(_), Bet::Pass(_)) => Err(CarryError::new(self, "invalid pass line bet with button established")),
      (None, Bet::Come(_)) => Err(CarryError::new(self, "invalid come bet without button established")),
      _ => Ok(self),
    };

    valid.and_then(|table| apply_bet(table, player, bet))
  }

  pub fn sit(self, player: &Player) -> Self {
    let Table {
      button,
      mut seats,
      rolls,
    } = self;

    seats.insert(player.id, Seat::with_balance(player.balance));
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

    let seats = self
      .seats
      .into_iter()
      .map(|(k, v)| {
        let Seat { bets, balance } = v;
        let start: (Vec<Bet>, u32) = (vec![], 0);

        let (bets, winnings) = bets.into_iter().fold(start, |(remaining, winnings), item| {
          let result = item.result(&roll);
          let winnings = winnings + result.winnings();
          let remaining = remaining.into_iter().chain(result.remaining()).collect();
          (remaining, winnings)
        });

        let balance = balance + winnings;
        (k, Seat { bets, balance })
      })
      .collect();

    let rolls = Some(roll)
      .into_iter()
      .chain(self.rolls.into_iter())
      .take(2)
      .collect::<Vec<Roll>>();

    Table { seats, rolls, button }
  }

  pub fn payouts(&self, _id: String) -> Vec<Bet> {
    return Vec::new();
  }
}

#[derive(Debug, Clone, Eq)]
pub struct Player {
  id: uuid::Uuid,
  balance: u32,
}

impl PartialEq for Player {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Player {
  pub fn can_bet(&self, bet: &Bet) -> bool {
    let weight = bet.weight();
    weight < self.balance
  }

  pub fn bet(self, bet: &Bet) -> Result<Self, CarryError<Self>> {
    let weight = bet.weight();

    if weight > self.balance {
      log::warn!("player attempted bet without sufficient funds");
      return Err(CarryError::new(self, "insufficient funds"));
    }

    Ok(Player {
      id: self.id,
      balance: self.balance - weight,
    })
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
