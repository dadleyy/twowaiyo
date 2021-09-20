use std::io::Result;

pub async fn bet(services: &crate::Services, job: &bankah::BetJob) -> Result<()> {
  log::debug!("processing bet, services {:?}", services.status().await);
  Ok(())
}
