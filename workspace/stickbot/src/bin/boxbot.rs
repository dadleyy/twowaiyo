use std::io::{Error, ErrorKind, Result};
use std::time::Duration;

use stickbot;

use bankah::jobs::TableJob;

const POP_CMD: kramer::Command<&'static str, &'static str> =
  kramer::Command::List::<_, &str>(kramer::ListCommand::Pop(
    kramer::Side::Left,
    stickbot::constants::STICKBOT_JOB_QUEUE,
    Some((None, 3)),
  ));

fn response_string(response: &kramer::ResponseValue) -> Option<String> {
  match response {
    kramer::ResponseValue::String(inner) => Some(inner.clone()),
    res => {
      log::warn!("strange response from job queue - {:?}", res);
      None
    }
  }
}

fn parse_pop(response: &kramer::ResponseValue) -> Option<TableJob> {
  response_string(&response).and_then(|contents| serde_json::from_str::<TableJob>(&contents).ok())
}

async fn pop_next(services: &stickbot::Services) -> Result<Option<TableJob>> {
  let result = match services.command(&POP_CMD).await {
    Err(error) => {
      log::warn!("unable to pop from bet queue - {}", error);
      return Err(Error::new(ErrorKind::Other, format!("{}", error)));
    }
    Ok(kramer::Response::Item(kramer::ResponseValue::Empty)) => {
      log::trace!("empty response from queue, moving on");
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

  log::trace!("result from pop - {:?}, attempting to deserialize", result);

  Ok(result.get(1).and_then(parse_pop))
}

async fn work(services: &stickbot::Services) -> Result<()> {
  let job = match pop_next(&services).await? {
    Some(job) => job,
    None => return Ok(()),
  };

  log::debug!("deserialized job from queue - {:?}", job);

  let id = job.id();
  let result = match &job {
    TableJob::Admin(inner) => stickbot::processors::admin::reindex(&services, &inner.job).await,
    TableJob::Bet(inner) => stickbot::processors::bet(&services, &inner.job).await,
    TableJob::Roll(inner) => stickbot::processors::roll(&services, &inner.job).await,
  };

  // Processors will return a Result<E, T>, where `E` can either represent a "fatal" error that is non-retryable or
  // an error that is retryable. If the job is retryable, re-enqueue.
  let output = match result {
    Ok(output) => serde_json::to_string(&output)?,
    Err(bankah::jobs::JobError::Retryable) => {
      let retry = job.retry().ok_or(Error::new(ErrorKind::Other, "no-retryable"))?;

      log::warn!("job failed, but is retryable, re-adding back to the queue");
      let serialized = serde_json::to_string(&retry)?;

      // TODO: consider refactoring this command construction into a reusable place; it is used here and in our bets
      // api route to push bet jobs onto our queue.
      let command = kramer::Command::List(kramer::ListCommand::Push(
        (kramer::Side::Right, kramer::Insertion::Always),
        stickbot::constants::STICKBOT_JOB_RESULTS,
        kramer::Arity::One(serialized),
      ));

      return services.command(&command).await.map(|_| ());
    }
    Err(bankah::jobs::JobError::Terminal(error)) => return Err(Error::new(ErrorKind::Other, error)),
  };

  log::debug!("job '{}' processed - {}", id, output);

  let sid = id.to_string();

  // Insert into our results hash the output from the processor.
  let sets = kramer::Command::Hashes(kramer::HashCommand::Set(
    stickbot::constants::STICKBOT_JOB_RESULTS,
    kramer::Arity::One((&sid, output.as_str())),
    kramer::Insertion::Always,
  ));

  services.command(&sets).await.map(|_| ())
}

async fn run(services: stickbot::Services) -> Result<()> {
  let delay = std::env::var(stickbot::constants::BOXBOT_DELAY_ENV)
    .ok()
    .and_then(|content| content.parse::<u64>().ok())
    .unwrap_or(0);

  log::debug!("entering processing loop (w/ delay {:?})", delay);

  loop {
    if let Err(error) = work(&services).await {
      log::warn!("unable to process - {}", error);
    }

    if delay > 0 {
      log::debug!("sleeping worker for {} millis", delay);
      async_std::task::sleep(Duration::from_millis(delay)).await;
    }
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
