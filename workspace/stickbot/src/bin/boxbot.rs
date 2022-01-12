use std::io::{Error, ErrorKind, Result};
use std::time::Duration;

use stickbot;

use bankah::jobs::{TableAdminJob, TableJob};

async fn work(services: &stickbot::Services) -> Result<()> {
  let job = match services.pop().await? {
    Some(job) => job,
    None => return Ok(()),
  };

  log::debug!("deserialized job from queue - {:?}", job);

  let id = job.id().to_string();
  let result = match &job {
    TableJob::Admin(inner) => match &inner.job {
      TableAdminJob::ReindexPopulations => stickbot::processors::admin::reindex(&services, &inner.job).await,
      TableAdminJob::CleanupPlayerData(id) => stickbot::processors::admin::cleanup(&services, &id).await,
    },
    TableJob::Bet(inner) => stickbot::processors::bet(&services, &inner.job).await,
    TableJob::Roll(inner) => stickbot::processors::roll(&services, &inner.job).await,
    TableJob::Sit(inner) => stickbot::processors::sit(&services, &inner.job).await,
    TableJob::Create(inner) => stickbot::processors::create(&services, &inner.job).await,
    TableJob::Stand(inner) => stickbot::processors::stand(&services, &inner.job).await,
  };

  let serialized = result
    .map(|inner| bankah::jobs::JobResult::wrap(job.id(), inner))
    .and_then(|out| {
      serde_json::to_string(&out).map_err(|error| {
        log::warn!("unable to serialze job output - {}", error);
        bankah::jobs::JobError::Terminal(format!("unable to serialze job output - {}", error))
      })
    });

  // Processors will return a Result<E, T>, where `E` can either represent a "fatal" error that is non-retryable or
  // an error that is retryable. If the job is retryable, re-enqueue.
  let output = match serialized {
    Ok(output) => output,
    Err(bankah::jobs::JobError::Retryable) => {
      let retry = job.retry().ok_or(Error::new(ErrorKind::Other, "no-retryable"))?;
      services.queue(&retry).await.map_err(|error| {
        log::warn!("unable to persist retry into queue - {}", error);
        error
      })?;
      log::debug!("job '{}' scheduled for retry", retry.id());
      return Ok(());
    }
    Err(bankah::jobs::JobError::Terminal(error)) => return Err(Error::new(ErrorKind::Other, error)),
  };

  log::debug!("job '{}' processed - {}", id, output);

  // TODO(service-coagulation): consider `services.finalize(&output)`?
  let storage = format!("{}", stickbot::env::JobStore::Results);

  // Insert into our results hash the output from the processor.
  let sets = kramer::Command::Hashes(kramer::HashCommand::Set(
    &storage,
    kramer::Arity::One((&id, output.as_str())),
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
