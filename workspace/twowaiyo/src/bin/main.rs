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

  let mut table = twowaiyo::Table::default();
  let player = twowaiyo::Player::default();

  log::debug!("initialized player {:?} and table {:?}", player, table);

  loop {
    log::info!("current table state - {:?}", table);
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
        log::info!("received roll, throwing!");
        table = table.roll();
      }
      None => log::warn!("unable to parse input, skipping"),
    }
  }

  return Ok(());
}
