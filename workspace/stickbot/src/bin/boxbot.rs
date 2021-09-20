use std::io::{Error, ErrorKind, Result};
use std::time::Duration;

use stickbot;

const POP_CMD: kramer::Command<&'static str, &'static str> =
  kramer::Command::List::<_, &str>(kramer::ListCommand::Pop(
    kramer::Side::Left,
    stickbot::constants::STICKBOT_BETS_QUEUE,
    Some((None, 3)),
  ));

fn parse_pop(response: &kramer::ResponseValue) -> Option<bankah::TableJob> {
  if let kramer::ResponseValue::String(inner) = response {
    let deserialized = serde_json::from_str::<bankah::TableJob>(&inner);
    return deserialized.ok();
  }

  None
}

async fn work(services: &stickbot::Services) -> Result<()> {
  let result = match services.command(&POP_CMD).await {
    Err(error) => {
      log::warn!("unable to pop from bet queue - {}", error);
      return Err(Error::new(ErrorKind::Other, format!("{}", error)));
    }
    Ok(kramer::Response::Item(kramer::ResponseValue::Empty)) => {
      log::debug!("nothing to pop off, sleeping and moving on");
      return Ok(());
    }
    Ok(kramer::Response::Array(values)) => values,
    Ok(kramer::Response::Error) => {
      log::warn!("unable to pop from queue - redis error");
      return Err(Error::new(ErrorKind::Other, "unkown-error"));
    }
    Ok(kramer::Response::Item(inner)) => {
      log::warn!("unknown response from pop - '{:?}'", inner);
      return Err(Error::new(ErrorKind::Other, format!("{:?}", inner)));
    }
  };

  log::debug!("result from pop - {:?}, attempting to deserialize", result);

  let job = result
    .get(1)
    .and_then(parse_pop)
    .ok_or(Error::new(ErrorKind::Other, "unrecognized-pop"))?;

  log::debug!("result from deserialize - {:?}", job);

  let (id, result) = match job {
    bankah::TableJob::Bet((id, bet)) => (id, stickbot::processors::bet(&services, &bet).await),
  };

  match result {
    Ok(_) => log::info!("job '{}' processed", id),
    Err(error) => log::warn!("unable to process job - {}", error),
  }

  let sets = kramer::Command::Hashes(kramer::HashCommand::Set(
    stickbot::constants::STICKBOT_BET_RESULTS,
    kramer::Arity::One((&id, "done")),
    kramer::Insertion::Always,
  ));

  services.command(&sets).await.map(|_| ())
}

async fn run(services: stickbot::Services) -> Result<()> {
  log::debug!("entering processing loop");

  loop {
    if let Err(error) = work(&services).await {
      log::warn!("unable to process - {}", error);
    }
    async_std::task::sleep(Duration::from_secs(10)).await;
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
