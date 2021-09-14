use super::{
  bets::Bet,
  errors::{CarryError, PlayerBetViolation, RuleViolation},
  roll::Roll,
};

#[derive(Clone, Default)]
pub struct Seat {
  bets: Vec<Bet>,
  balance: u32,
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
