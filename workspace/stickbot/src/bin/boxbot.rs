use std::io::Result;
use std::time::Duration;

use stickbot;

const POP_CMD: kramer::Command<&'static str, &'static str> =
  kramer::Command::List::<_, &str>(kramer::ListCommand::Pop(
    kramer::Side::Left,
    stickbot::constants::STICKBOT_BETS_QUEUE,
    Some((None, 3)),
  ));

async fn run(services: stickbot::Services) -> Result<()> {
  log::debug!("entering processing loop");

  loop {
    let result = match services.command(&POP_CMD).await {
      Err(error) => {
        log::warn!("unable to pop from bet queue - {}", error);
        continue;
      }
      Ok(kramer::Response::Item(kramer::ResponseValue::Empty)) => {
        log::debug!("nothing to pop off, sleeping and moving on");
        async_std::task::sleep(Duration::from_secs(2)).await;
        continue;
      }
      Ok(kramer::Response::Array(values)) => values,
      Ok(kramer::Response::Error) => {
        log::warn!("unable to pop from queue - redis error");
        async_std::task::sleep(Duration::from_secs(2)).await;
        continue;
      }
      Ok(kramer::Response::Item(inner)) => {
        log::warn!("unknown response from pop - '{:?}'", inner);
        continue;
      }
    };

    log::debug!("result from pop - {:?}, attempting to deserialize", result);

    let serialized = match result.get(1) {
      Some(kramer::ResponseValue::String(value)) => value,
      other => {
        log::warn!("strange response entry - {:?}", other);
        continue;
      }
    };

    let deserialized = serde_json::from_str::<bankah::TableJob>(&serialized);
    log::debug!("result from deserialize - {:?}", deserialized);
    async_std::task::sleep(Duration::from_secs(10)).await;
    log::debug!("polling bet queue");
  }
}

fn main() -> Result<()> {
  dotenv::dotenv().expect("unable to load environment from '.env'");
  env_logger::init();

  log::info!("environment ready, booting");

  async_std::task::block_on(async {
    let services = stickbot::Services::new().await?;
    run(services).await
  })
}
