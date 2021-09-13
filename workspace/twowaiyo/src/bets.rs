use super::roll::Roll;

#[derive(Debug, PartialEq, Clone)]
pub enum BetResult<T> {
  Noop(T),
  Win(u32),
  Loss,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RaceBet {
  amount: u32,
  target: Option<u8>,
}

impl RaceBet {
  pub fn result(self, roll: &Roll) -> BetResult<Self> {
    let total = roll.total();

    match (self.target, total) {
      (Some(goal), value) if value == goal => BetResult::Win(10),
      (Some(_), 7) => BetResult::Loss,
      (Some(goal), _) => BetResult::Noop(RaceBet {
        amount: self.amount,
        target: Some(goal),
      }),

      (None, 7) | (None, 11) => BetResult::Win(self.amount),
      (None, 2) | (None, 3) | (None, 12) => BetResult::Loss,
      (None, value) => BetResult::Noop(RaceBet {
        amount: self.amount,
        target: Some(value),
      }),
    }
  }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Bet {
  Pass(RaceBet),
  PassOdds(u32, u8),

  Come(RaceBet),
  ComeOdds(u32, u8),

  Place(u32, u8),

  Field(u32),
}

impl Bet {
  pub fn start_come(amount: u32) -> Self {
    Bet::Come(RaceBet { amount, target: None })
  }

  pub fn start_pass(amount: u32) -> Self {
    Bet::Pass(RaceBet { amount, target: None })
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
  use super::{BetResult, RaceBet};
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
    assert_eq!(bet.result(&roll), BetResult::Win(10),);
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
    assert_eq!(bet.result(&roll), BetResult::Win(10),);
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
    assert_eq!(bet.result(&roll), BetResult::Win(10));
  }

  #[test]
  fn test_hit_race_on_five() {
    let bet = RaceBet {
      amount: 10,
      target: Some(5),
    };
    let roll = vec![2u8, 3u8].into_iter().collect::<Roll>();
    assert_eq!(bet.result(&roll), BetResult::Win(10));
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
}
