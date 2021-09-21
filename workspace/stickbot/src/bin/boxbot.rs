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

async fn pop_next(services: &stickbot::Services) -> Result<Option<bankah::TableJob>> {
  let result = match services.command(&POP_CMD).await {
    Err(error) => {
      log::warn!("unable to pop from bet queue - {}", error);
      return Err(Error::new(ErrorKind::Other, format!("{}", error)));
    }
    Ok(kramer::Response::Item(kramer::ResponseValue::Empty)) => {
      log::debug!("nothing to pop off");
      return Ok(None);
    }
    Ok(kramer::Response::Array(values)) => values,
    Ok(kramer::Response::Error) => {
      log::warn!("unable to pop from queue - redis error");
      return Err(Error::new(ErrorKind::Other, "invalid-response"));
    }
    Ok(kramer::Response::Item(inner)) => {
      log::warn!("unknown response from pop - '{:?}'", inner);
      return Err(Error::new(ErrorKind::Other, format!("{:?}", inner)));
    }
  };

  log::debug!("result from pop - {:?}, attempting to deserialize", result);

  Ok(result.get(1).and_then(parse_pop))
}

async fn work(services: &stickbot::Services) -> Result<()> {
  let job = match pop_next(&services).await? {
    Some(job) => job,
    None => return Ok(()),
  };

  log::debug!("result from deserialize - {:?}", job);

  let (id, result) = match &job {
    bankah::TableJob::Bet(inner) => (inner.id.clone(), stickbot::processors::bet(&services, &inner.job).await),
    bankah::TableJob::Roll(inner) => (
      inner.id.clone(),
      stickbot::processors::roll(&services, &inner.job).await,
    ),
  };

  // Processors will return a Result<E, T>, where `E` can either represent a "fatal" error that is non-retryable or
  // an error that is retryable. If the job is retryable, re-enqueue.
  let output = match result {
    Ok(output) => serde_json::to_string(&output)?,
    Err(bankah::JobError::Retryable) => {
      let retry = job.retry().ok_or(Error::new(ErrorKind::Other, "no-retryable"))?;

      log::warn!("job failed, but is retryable, re-adding back to the queue");
      let serialized = serde_json::to_string(&retry)?;

      // TODO: consider refactoring this command construction into a reusable place; it is used here and in our bets
      // api route to push bet jobs onto our queue.
      let command = kramer::Command::List(kramer::ListCommand::Push(
        (kramer::Side::Right, kramer::Insertion::Always),
        stickbot::constants::STICKBOT_BETS_QUEUE,
        kramer::Arity::One(serialized),
      ));

      return services.command(&command).await.map(|_| ());
    }
    Err(bankah::JobError::Terminal(error)) => return Err(Error::new(ErrorKind::Other, error)),
  };

  log::debug!("job '{}' processed - {}", id, output);

  // Insert into our results hash the output from the processor.
  let sets = kramer::Command::Hashes(kramer::HashCommand::Set(
    stickbot::constants::STICKBOT_BET_RESULTS,
    kramer::Arity::One((&id, output.as_str())),
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

    async_std::task::sleep(Duration::from_secs(2)).await;
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
