use bankah;

pub async fn bet(services: &crate::Services, job: &bankah::BetJob) -> Result<bankah::TableJobOutput, bankah::JobError> {
  log::debug!("processing bet, services {:?}", services.status().await);
  Ok(bankah::TableJobOutput::BetProcessed)
}
