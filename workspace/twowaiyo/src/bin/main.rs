use std::io::{stdin, Result};

use dotenv;

use twowaiyo;

fn main() -> Result<()> {
  if let Err(error) = dotenv::dotenv() {
    println!("unable to load environment - {:?}", error);
    return Ok(());
  }

  env_logger::init();
  log::info!("logger initialized, preparing table");

  let table = twowaiyo::Table::default();
  let player = twowaiyo::Player::default();
  let mut dealer = twowaiyo::Dealer::new(table, player);

  log::debug!("initialized state {:?}", dealer);

  loop {
    log::info!("{:?}", dealer);

    let mut buffer = String::with_capacity(32);

    match stdin().read_line(&mut buffer) {
      Err(error) => {
        log::warn!("unable to read from stdin - {:?}", error);
        continue;
      }
      Ok(size) => log::debug!("successfully read {} bytes from stdin", size),
    };

    let action = twowaiyo::io::Action::parse(buffer.trim());

    match action {
      Some(twowaiyo::io::Action::Exit) => {
        log::info!("received exit, leaving main game loop");
        break;
      }

      Some(twowaiyo::io::Action::Roll) => {
        log::info!("throwing die...");
        dealer = dealer.roll();
      }
      Some(twowaiyo::io::Action::Bet(bet)) => {
        log::info!("attempting bet - {:?}", bet);

        dealer = dealer
          .bet(&bet)
          .map_err(|carry| {
            log::warn!("invalid bet - {:?}", carry);
            carry.consume()
          })
          .unwrap_or_else(|d| d);
      }

      None => log::warn!("unable to parse input, skipping"),
    }
  }

  return Ok(());
}
