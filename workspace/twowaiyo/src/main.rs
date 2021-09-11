use dotenv;
use log::info;
use std::io::Result;

fn main() -> Result<()> {
  if let Err(error) = dotenv::dotenv() {
    println!("unable to load environment - {:?}", error);
    return Ok(());
  }

  env_logger::init();

  info!("Hello, world!");
  return Ok(());
}
