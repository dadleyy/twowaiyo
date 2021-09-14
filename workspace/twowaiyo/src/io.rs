use std::str::FromStr;

use super::{bets::Bet, checks, roll::Hardway};

#[derive(Debug)]
pub enum Action {
  Exit,
  Roll,
  Bet(Bet),
}

fn log_pass<E>(error: E) -> E
where
  E: std::fmt::Display,
{
  log::warn!("unable to parse input - {}", error);
  error
}

fn parse_bet_line(parts: &Vec<&str>) -> Option<Action> {
  log::debug!("parsing bet string - '{:?}'", parts);

  match parts[..] {
    ["bet", "field", value] => {
      log::debug!("parsing field bet - {}", value);

      u32::from_str(value)
        .map_err(log_pass)
        .ok()
        .map(Bet::Field)
        .map(Action::Bet)
    }

    ["bet", "place", target, value] => {
      log::debug!("parsing come line bet - {}", value);
      let parsed_target =
        u8::from_str(target)
          .map_err(log_pass)
          .ok()
          .and_then(|value| if checks::is_place(value) { Some(value) } else { None });
      let parsed_value = u32::from_str(value).map_err(log_pass).ok();

      parsed_target
        .zip(parsed_value)
        .map(|(target, value)| Bet::Place(value, target))
        .map(Action::Bet)
    }

    ["bet", "hardway", "four", value] => u32::from_str(value)
      .map_err(log_pass)
      .ok()
      .map(|amount| Bet::Hardway(amount, Hardway::Four))
      .map(Action::Bet),

    ["bet", "hardway", "six", value] => u32::from_str(value)
      .map_err(log_pass)
      .ok()
      .map(|amount| Bet::Hardway(amount, Hardway::Six))
      .map(Action::Bet),

    ["bet", "hardway", "eight", value] => u32::from_str(value)
      .map_err(log_pass)
      .ok()
      .map(|amount| Bet::Hardway(amount, Hardway::Eight))
      .map(Action::Bet),

    ["bet", "hardway", "ten", value] => u32::from_str(value)
      .map_err(log_pass)
      .ok()
      .map(|amount| Bet::Hardway(amount, Hardway::Ten))
      .map(Action::Bet),

    ["bet", "come", value] => {
      log::debug!("parsing come line bet - {}", value);

      u32::from_str(value)
        .map_err(log_pass)
        .ok()
        .map(|amount| Bet::start_come(amount))
        .map(Action::Bet)
    }

    ["bet", "come-odds", target, value] => {
      log::debug!("parsing come odds - {} on {}", value, target);

      let parsed_amount = u32::from_str(value).map_err(log_pass).ok();
      let parsed_target = u8::from_str(target).map_err(log_pass).ok();

      parsed_amount
        .zip(parsed_target)
        .map(|(amount, target)| Bet::ComeOdds(amount, target))
        .map(Action::Bet)
    }

    ["bet", "pass-odds", value] => {
      log::debug!("parsing pass odds - {}", value);

      u32::from_str(value)
        .map_err(log_pass)
        .ok()
        .map(|amount| Bet::PassOdds(amount, 0))
        .map(Action::Bet)
    }

    ["bet", "pass", value] => {
      log::debug!("parsing pass line bet - {}", value);

      u32::from_str(value)
        .map_err(log_pass)
        .ok()
        .map(|amount| Bet::start_pass(amount))
        .map(Action::Bet)
    }

    _ => {
      log::debug!("unrecognized bet - {:?}", parts);
      None
    }
  }
}

impl Action {
  pub fn parse<T>(input: T) -> Option<Self>
  where
    T: std::fmt::Display,
  {
    let nice = format!("{}", input);

    match nice.as_str() {
      "" => Some(Action::Roll),
      "exit" => Some(Action::Exit),
      "roll" => Some(Action::Roll),

      bet if bet.starts_with("bet") => {
        let parts = bet.split(" ").collect::<Vec<&str>>();
        parse_bet_line(&parts)
      }

      _ => None,
    }
  }
}
