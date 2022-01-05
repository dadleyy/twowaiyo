use serde::Serialize;

use crate::db;
use crate::web::{cookie as get_cookie, Body, Error, Request, Response, Result};

#[derive(Serialize)]
struct DeletionResponsePayload {
  id: String,
}

pub async fn delete(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-cook"))?;
  let player = request
    .state()
    .authority(cookie.value())
    .await
    .and_then(|authority| authority.player())
    .ok_or(Error::from_str(404, ""))?;

  let players = request.state().players();

  log::debug!("player {} deleting account", player.id);

  players
    .delete_one(db::doc! { "id": player.id.to_string() }, None)
    .await
    .map_err(|error| {
      log::warn!("unable to delete player record: {}", error);
      error
    })?;

  log::debug!("player document '{}' deleted, queuing cleanup job", player.id);

  let job = bankah::jobs::TableJob::admin(bankah::jobs::TableAdminJob::CleanupPlayerData(player.id.to_string()));
  request.state().queue(&job).await.map_err(|error| {
    log::warn!("unable to schedule player cleanup job - {}", error);
    error
  })?;

  Body::from_json(&DeletionResponsePayload {
    id: player.id.to_string(),
  })
  .map(|body| Response::builder(200).body(body).build())
}
