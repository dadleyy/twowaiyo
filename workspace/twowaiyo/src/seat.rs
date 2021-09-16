use super::{
  bets::Bet,
  errors::{CarryError, PlayerBetViolation, RuleViolation},
  roll::Roll,
};

#[derive(Clone, Default, PartialEq)]
pub struct Seat {
  bets: Vec<Bet>,
  balance: u32,
}

impl From<&bankah::SeatState> for Seat {
  fn from(seat: &bankah::SeatState) -> Seat {
    let bets = seat.bets.iter().map(|b| b.into()).collect();

    Seat {
      bets,
      balance: seat.balance,
    }
  }
}

impl From<&Seat> for bankah::SeatState {
  fn from(seat: &Seat) -> bankah::SeatState {
    bankah::SeatState {
      balance: seat.balance,
      bets: seat.bets.iter().map(|b| b.into()).collect(),
    }
  }
}

impl std::fmt::Debug for Seat {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    writeln!(formatter, "balance: {}", self.balance)?;
    writeln!(formatter, "bets:")?;

    for bet in &self.bets {
      writeln!(formatter, "  {:?}", bet)?;
    }

    Ok(())
  }
}

impl Seat {
  pub fn with_balance(balance: u32) -> Self {
    Seat {
      balance,
      ..Self::default()
    }
  }

  pub fn stand(self) -> (u32, Option<Self>) {
    let Seat { bets, balance } = self;
    let start = (balance, Vec::with_capacity(bets.len()));
    let (balance, bets) = bets.into_iter().fold(start, |(balance, bets), bet| {
      let (amt, rem) = bet.pull();
      let bets = bets.into_iter().chain(rem).collect();
      (balance + amt, bets)
    });

    if bets.len() == 0 {
      return (balance, None);
    }

    (balance, Some(Seat { bets, balance: 0 }))
  }

  pub fn roll(self, roll: &Roll) -> Self {
    let Seat { bets, balance } = self;
    let start: (Vec<Bet>, u32) = (vec![], 0);

    let (bets, winnings) = bets.into_iter().fold(start, |(remaining, winnings), item| {
      let result = item.result(&roll);
      log::info!("{:<25} -> {:<25}", format!("{:?}", item), format!("{:?}", result));
      let winnings = winnings + result.winnings();
      let remaining = remaining.into_iter().chain(result.remaining()).collect();
      (remaining, winnings)
    });

    Seat {
      bets,
      balance: balance + winnings,
    }
  }

  pub fn bet(self, bet: &Bet) -> Result<Self, CarryError<Self>> {
    self
      .normalize_bet(bet)
      .map(|bet| {
        let balance = self.balance - bet.weight();
        let bets = self.bets.iter().chain(Some(&bet)).map(|b| b.clone()).collect();
        Seat { balance, bets }
      })
      .map_err(|error| {
        let seat = Seat { ..self };
        CarryError::new(seat, RuleViolation::PlayerBetViolation(error))
      })
  }

  fn normalize_bet(&self, bet: &Bet) -> Result<Bet, PlayerBetViolation> {
    let weight = bet.weight();

    if weight > self.balance {
      return Err(PlayerBetViolation::InsufficientFunds);
    }

    match bet {
      Bet::PassOdds(amount, _) => {
        log::debug!("pass odds received, checking match");
        self
          .bets
          .iter()
          .find_map(|b| b.pass_target())
          .map(|target| Bet::PassOdds(*amount, target))
          .ok_or(PlayerBetViolation::MissingPassForOdds)
      }

      Bet::ComeOdds(amount, target) => {
        log::debug!("pass odds received, checking match");

        self
          .bets
          .iter()
          .find_map(|b| {
            b.come_target()
              .and_then(|inner| if inner == *target { Some(target) } else { None })
          })
          .map(|target| Bet::ComeOdds(*amount, *target))
          .ok_or(PlayerBetViolation::MissingComeForOdds)
      }

      _ => Ok(bet.clone()),
    }
  }
}

#[cfg(test)]
mod test {
  use super::Seat;
  use crate::bets::Bet;

  #[test]
  fn stand_with_nothing() {
    let seat = Seat::with_balance(100);
    assert_eq!(seat.stand(), (100u32, None::<Seat>));
  }

  #[test]
  fn stand_with_pass_off() {
    let seat = Seat::with_balance(100);
    let seat = seat.bet(&Bet::start_pass(50)).expect("");
    assert_eq!(seat.stand(), (100u32, None));
  }

  #[test]
  fn stand_with_pass_on() {
    let seat = Seat::with_balance(100);
    let seat = seat.bet(&Bet::start_pass(50)).expect("");
    let roll = vec![2u8, 4u8].into_iter().collect();
    let seat = seat.roll(&roll);
    let expected = Seat::with_balance(50)
      .bet(&Bet::start_pass(50).result(&roll).remaining().unwrap())
      .unwrap();
    assert_eq!(seat.stand(), (50u32, Some(expected)));
  }
}
