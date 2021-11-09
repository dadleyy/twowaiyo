use super::{
  bets::{Bet, BetResult},
  errors::{CarryError, PlayerBetViolation, RuleViolation},
  roll::Roll,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SeatRuns {
  wins: Vec<(Bet, u32)>,
  losses: Vec<(Bet, u32)>,
}

impl SeatRuns {
  pub fn losses(&self) -> u32 {
    self.losses.iter().fold(0, |acc, item| acc + item.1)
  }

  pub fn winnings(&self) -> u32 {
    self.wins.iter().fold(0, |acc, item| acc + item.1)
  }
}

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

  pub fn roll(self, roll: &Roll) -> (Self, SeatRuns) {
    let Seat { bets, balance } = self;
    let start: (Vec<Bet>, _) = (vec![], SeatRuns::default());

    let (stays, runs) = bets.into_iter().fold(start, |(stays, runs), item| {
      let result = item.result(&roll);
      log::info!("{:<25} -> {:<25}", format!("{:?}", item), format!("{:?}", result));
      let runs = match &result {
        BetResult::Win(amount) => SeatRuns {
          losses: runs.losses,
          wins: runs.wins.into_iter().chain(Some((item, amount + 0))).collect(),
        },
        BetResult::Loss(amount) => SeatRuns {
          wins: runs.wins,
          losses: runs.losses.into_iter().chain(Some((item, amount + 0))).collect(),
        },
        BetResult::Noop(_) => runs,
      };

      (stays.into_iter().chain(result.remaining()).collect(), runs)
    });

    let next = Seat {
      balance: balance + runs.winnings(),
      bets: stays,
    };

    return (next, runs);
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
  use super::{Seat, SeatRuns};
  use crate::bets::Bet;

  #[test]
  fn run_with_winners() {
    let seat = Seat::with_balance(100);
    let seat = seat.bet(&Bet::start_pass(10)).expect("");
    let roll = vec![2u8, 5u8].into_iter().collect();
    let expected = SeatRuns {
      wins: vec![(Bet::start_pass(10), 20)],
      losses: vec![],
    };
    assert_eq!(seat.roll(&roll), (Seat::with_balance(110), expected));
  }

  #[test]
  fn run_with_losers() {
    let seat = Seat::with_balance(100);
    let seat = seat.bet(&Bet::start_pass(10)).expect("");
    let roll = vec![2u8, 1u8].into_iter().collect();
    let expected = SeatRuns {
      wins: vec![],
      losses: vec![(Bet::start_pass(10), 10)],
    };
    assert_eq!(seat.roll(&roll), (Seat::with_balance(90), expected));
  }

  #[test]
  fn run_with_losers_after_pass() {
    let seat = Seat::with_balance(100);
    let seat = seat.bet(&Bet::start_pass(10)).expect("");
    let roll = vec![2u8, 4u8].into_iter().collect();
    let passed = seat.roll(&roll).0;
    let crapped = vec![2u8, 5u8].into_iter().collect();
    let expected = SeatRuns {
      wins: vec![],
      losses: vec![(Bet::start_pass(10).result(&roll).remaining().unwrap(), 10)],
    };
    assert_eq!(passed.roll(&crapped), (Seat::with_balance(90), expected));
  }

  #[test]
  fn run_with_winners_after_pass() {
    let seat = Seat::with_balance(100);
    let seat = seat.bet(&Bet::start_pass(10)).expect("");
    let roll = vec![2u8, 4u8].into_iter().collect();
    let passed = seat.roll(&roll).0;
    let hit = vec![2u8, 4u8].into_iter().collect();
    let expected = SeatRuns {
      losses: vec![],
      wins: vec![(Bet::start_pass(10).result(&roll).remaining().unwrap(), 20)],
    };
    assert_eq!(passed.roll(&hit), (Seat::with_balance(110), expected));
  }

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
    let seat = seat.roll(&roll).0;
    let expected = Seat::with_balance(50)
      .bet(&Bet::start_pass(50).result(&roll).remaining().unwrap())
      .unwrap();
    assert_eq!(seat.stand(), (50u32, Some(expected)));
  }
}
