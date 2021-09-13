use super::roll::Roll;

#[derive(Debug, PartialEq, Clone)]
pub enum BetResult<T> {
  Noop(T),
  Win(u32),
  Loss,
}

impl<T> BetResult<T> {
  pub fn map<F, U>(self, mapper: F) -> BetResult<U>
  where
    F: Fn(T) -> U,
  {
    match self {
      BetResult::Win(amount) => BetResult::Win(amount),
      BetResult::Loss => BetResult::Loss,
      BetResult::Noop(item) => BetResult::Noop(mapper(item)),
    }
  }

  pub fn winnings(&self) -> u32 {
    match self {
      BetResult::Win(amount) => *amount,
      BetResult::Loss => 0,
      BetResult::Noop(_) => 0,
    }
  }

  pub fn remaining(self) -> Option<T> {
    match self {
      BetResult::Win(_) => None,
      BetResult::Loss => None,
      BetResult::Noop(item) => Some(item),
    }
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RaceBet {
  amount: u32,
  target: Option<u8>,
}

impl RaceBet {
  pub fn result(&self, roll: &Roll) -> BetResult<Self> {
    let total = roll.total();

    match (self.target, total) {
      (Some(goal), value) if value == goal => BetResult::Win(self.amount + self.amount),
      (Some(_), 7) => BetResult::Loss,
      (Some(goal), _) => BetResult::Noop(RaceBet {
        amount: self.amount,
        target: Some(goal),
      }),

      (None, 7) | (None, 11) => BetResult::Win(self.amount + self.amount),
      (None, 2) | (None, 3) | (None, 12) => BetResult::Loss,
      (None, value) => BetResult::Noop(RaceBet {
        amount: self.amount,
        target: Some(value),
      }),
    }
  }
}

#[derive(PartialEq, Clone)]
pub enum Bet {
  Pass(RaceBet),
  PassOdds(u32, u8),

  Come(RaceBet),
  ComeOdds(u32, u8),

  Place(u32, u8),

  Field(u32),
}

impl std::fmt::Debug for Bet {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Bet::Pass(race) => write!(formatter, "pass[{} on {:?}]", race.amount, race.target),
      Bet::Come(race) => write!(formatter, "come[{} on {:?}]", race.amount, race.target),
      Bet::PassOdds(amount, target) => write!(formatter, "pass-odds[{} on {}]", amount, target),
      Bet::ComeOdds(amount, target) => write!(formatter, "come-odds[{} on {}]", amount, target),
      Bet::Field(amount) => write!(formatter, "field[{}]", amount),
      Bet::Place(amount, target) => write!(formatter, "place[{} on {}]", amount, target),
    }
  }
}

fn odds_result(total: u8, target: u8, wager: u32) -> BetResult<(u32, u8)> {
  if total == 7 {
    return BetResult::Loss;
  }

  if total != target {
    return BetResult::Noop((wager, target));
  }

  match target {
    4 | 10 => BetResult::Win((wager * 2) + wager),
    5 | 9 => {
      let half = wager / 2;
      BetResult::Win((wager + half) + wager)
    }
    6 | 8 => {
      let fifth = wager / 5;
      BetResult::Win((wager + fifth) + wager)
    }
    _ => BetResult::Noop((wager, target)),
  }
}

impl Bet {
  pub fn start_come(amount: u32) -> Self {
    Bet::Come(RaceBet { amount, target: None })
  }

  pub fn start_pass(amount: u32) -> Self {
    Bet::Pass(RaceBet { amount, target: None })
  }

  pub fn come_target(&self) -> Option<u8> {
    match self {
      Bet::Come(race) => race.target,
      _ => None,
    }
  }

  pub fn pass_target(&self) -> Option<u8> {
    match self {
      Bet::Pass(race) => race.target,
      _ => None,
    }
  }

  pub fn result(&self, roll: &Roll) -> BetResult<Self> {
    let total = roll.total();

    match self {
      Bet::Pass(race) => race.result(roll).map(Bet::Pass),
      Bet::Come(race) => race.result(roll).map(Bet::Come),
      Bet::PassOdds(amount, target) => {
        odds_result(total, *target, *amount).map(|(amount, target)| Bet::PassOdds(amount, target))
      }
      Bet::ComeOdds(amount, target) => {
        odds_result(total, *target, *amount).map(|(amount, target)| Bet::ComeOdds(amount, target))
      }
      Bet::Place(amount, target) => {
        odds_result(total, *target, *amount).map(|(amount, target)| Bet::Place(amount, target))
      }
      Bet::Field(amount) => match total {
        2 | 12 => BetResult::Win((amount * 2) + amount),
        3 | 4 | 9 | 10 | 11 => BetResult::Win(amount + amount),
        _ => BetResult::Loss,
      },
    }
  }

  pub fn weight(&self) -> u32 {
    match self {
      Bet::Pass(inner) => inner.amount,
      Bet::PassOdds(amount, _) => *amount,

      Bet::Come(inner) => inner.amount,
      Bet::ComeOdds(amount, _) => *amount,

      Bet::Place(amount, _) => *amount,
      Bet::Field(amount) => *amount,
    }
  }
}

#[cfg(test)]
mod test {
  use super::{Bet, BetResult, RaceBet};
  use crate::roll::Roll;

  #[test]
  fn test_hit_race_off_two() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![1u8, 1u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Loss);
  }

  #[test]
  fn test_hit_race_off_three() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![1u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Loss);
  }

  #[test]
  fn test_hit_race_off_four() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![2u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(4u8)
      })
    );
  }

  #[test]
  fn test_hit_race_off_five() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![2u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(5u8)
      })
    );
  }

  #[test]
  fn test_hit_race_off_six() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![3u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(6u8)
      })
    );
  }

  #[test]
  fn test_hit_race_off_seven() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![3u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(20));
  }

  #[test]
  fn test_hit_race_off_eight() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![4u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(8)
      })
    );
  }

  #[test]
  fn test_hit_race_off_nine() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![4u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(9)
      })
    );
  }

  #[test]
  fn test_hit_race_off_ten() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![4u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(10)
      })
    );
  }

  #[test]
  fn test_hit_race_off_eleven() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![5u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(20));
  }

  #[test]
  fn test_hit_race_off_twelve() {
    let bet = RaceBet {
      amount: 10,
      target: None,
    };
    let roll = vec![6u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Loss);
  }

  // ON

  #[test]
  fn test_hit_race_on_two() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let expected = bet.clone();
    let roll = vec![1u8, 1u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Noop(expected));
  }

  #[test]
  fn test_hit_race_on_three() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let expected = bet.clone();
    let roll = vec![1u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Noop(expected));
  }

  #[test]
  fn test_hit_race_on_four() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![2u8, 2u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(20));
  }

  #[test]
  fn test_hit_race_on_five() {
    let bet = RaceBet {
      amount: 10,
      target: Some(5),
    };
    let roll = vec![2u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(20));
  }

  #[test]
  fn test_hit_race_on_six() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![3u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(4)
      })
    );
  }

  #[test]
  fn test_hit_race_on_seven() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![3u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Loss);
  }

  #[test]
  fn test_hit_race_on_eight() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![4u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(4)
      })
    );
  }

  #[test]
  fn test_hit_race_on_nine() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![4u8, 5u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(4)
      })
    );
  }

  #[test]
  fn test_hit_race_on_ten() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![4u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(4)
      })
    );
  }

  #[test]
  fn test_hit_race_on_eleven() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let roll = vec![5u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(RaceBet {
        amount: 10,
        target: Some(4)
      })
    );
  }

  #[test]
  fn test_hit_race_on_twelve() {
    let bet = RaceBet {
      amount: 10,
      target: Some(4),
    };
    let expected = bet.clone();
    let roll = vec![6u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Noop(expected));
  }

  #[test]
  fn test_start_pass_fail() {
    let bet = Bet::start_pass(100);
    let roll = vec![6u8, 6u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Loss);
  }

  #[test]
  fn test_start_pass_button() {
    let bet = Bet::start_pass(100);
    let roll = vec![6u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(
      bet.result(&roll),
      BetResult::Noop(Bet::Pass(RaceBet {
        amount: 100,
        target: Some(10)
      }))
    );
  }

  #[test]
  fn test_place_fail() {
    let bet = Bet::Place(100, 10);
    let roll = vec![3u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Loss);
  }

  #[test]
  fn test_place_win_ten() {
    let bet = Bet::Place(30, 10);
    let roll = vec![6u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(90));
  }

  #[test]
  fn test_place_win_nine() {
    let bet = Bet::Place(30, 9);
    let roll = vec![6u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(75));
  }

  #[test]
  fn test_place_win_eight() {
    let bet = Bet::Place(5, 8);
    let roll = vec![4u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(11));
  }

  #[test]
  fn test_place_noop() {
    let bet = Bet::Place(100, 10);
    let roll = vec![2u8, 4u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Noop(Bet::Place(100, 10)));
  }
}
