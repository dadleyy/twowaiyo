use serde::{Deserialize, Serialize};
use surf;

use crate::auth;
use crate::constants;
use crate::db;
use crate::web::{cookie as get_cookie, Body, Error, Redirect, Request, Response, Result, Url};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserInfo {
  pub sub: String,
  pub nickname: String,
  pub picture: String,
}

pub async fn fetch_user<T>(token: T) -> Option<UserInfo>
where
  T: std::fmt::Display,
{
  let uri = std::env::var(constants::AUTH_O_USERINFO_URI_ENV).unwrap_or_default();

  let mut res = surf::get(&uri)
    .header("Authorization", format!("Bearer {}", token))
    .await
    .ok()?;

  if res.status() != surf::StatusCode::Ok {
    log::warn!("bad response status - '{:?}'", res.status());
    return None;
  }

  log::debug!("loaded info with status '{}', attempting to parse", res.status());
  res.body_json::<UserInfo>().await.ok()
}

#[derive(Debug, Serialize)]
struct AuthCodeRequest {
  grant_type: String,
  client_id: String,
  client_secret: String,
  redirect_uri: String,
  code: String,
}

#[derive(Debug, Deserialize)]
struct AuthCodeResponse {
  access_token: String,
}

impl Default for AuthCodeRequest {
  fn default() -> Self {
    let client_id = std::env::var(constants::AUTH_O_CLIENT_ID_ENV).ok().unwrap_or_default();
    let redirect_uri = std::env::var(constants::AUTH_O_REDIRECT_URI_ENV)
      .ok()
      .unwrap_or_default();
    let client_secret = std::env::var(constants::AUTH_O_CLIENT_SECRET_ENV)
      .ok()
      .unwrap_or_default();

    AuthCodeRequest {
      client_id,
      client_secret,
      redirect_uri,
      code: "".into(),
      grant_type: "authorization_code".into(),
    }
  }
}

const COOKIE_FLAGS: &'static str = "Max-Age: 600; Path=/; SameSite=Strict; HttpOnly";

async fn token_from_response(response: &mut surf::Response) -> Option<String> {
  let status = response.status();

  match status {
    surf::StatusCode::Ok => log::debug!("good response from auth provider token api"),
    other => {
      log::warn!("bad status code from token response - '{:?}'", other);
      return None;
    }
  };

  response
    .body_json::<AuthCodeResponse>()
    .await
    .ok()
    .map(|body| body.access_token)
}

// Return our persisted player information from the token provided in our cookie.
pub async fn identify(request: Request) -> Result {
  let cookie = get_cookie(&request).ok_or(Error::from_str(404, "no-session"))?;
  let authority = request
    .state()
    .authority(cookie.value())
    .await
    .ok_or(Error::from_str(404, "no-user"))?
    .player()
    .ok_or(Error::from_str(404, "no-player"))?;

  log::info!("loaded authority - {:?}", authority);

  Body::from_json(&authority)
    .map(|body| Response::builder(200).body(body).build())
    .map_err(|error| {
      log::warn!("unable to serialize player authority - {}", error);
      Error::from_str(500, "")
    })
}

fn mkplayer(userinfo: &UserInfo) -> db::bson::Document {
  let id = uuid::Uuid::new_v4();
  db::doc! {
    "$setOnInsert": {
      "id": id.to_string(),
      "oid": userinfo.sub.clone(),
      "nickname": userinfo.nickname.clone(),
      "balance": 10000,
    }
  }
}

// Complete the oAuth authentication. Here we are receiving a code sent from our oAuth provider in the url and
// exchanging that for an authentication token. Assuming that goes well, we will either create or update the player
// record in our data store.
pub async fn complete(request: Request) -> Result {
  log::info!("completing auth flow");

  let code = request
    .url()
    .query_pairs()
    .find_map(|(k, v)| if k == "code" { Some(v) } else { None })
    .ok_or(Error::from_str(404, "no-code"))?;

  // Attempt top exchange our code with the oAuth provider for a token.
  let payload = AuthCodeRequest {
    code: code.into(),
    ..AuthCodeRequest::default()
  };
  let destination = std::env::var(constants::AUTH_O_TOKEN_URI_ENV).unwrap_or_default();
  log::info!("exchanging code - {} (at {})", payload.code, destination);
  let mut response = surf::post(&destination).body_json(&payload)?.await?;
  let token = token_from_response(&mut response)
    .await
    .ok_or(Error::from_str(404, "token-exchange"))?;

  // With our token, attempt to load the user info.
  log::info!("created token, loading user info");
  let user = fetch_user(&token).await.ok_or(Error::from_str(404, ""))?;
  log::info!("user loaded - '{:?}', finding or creating record", user);

  // With our loaded user data, attempt to store a new record in our players collection.
  let collection = request
    .state()
    .collection::<bankah::PlayerState, _>(constants::MONGO_DB_PLAYER_COLLECTION_NAME);

  let options = db::FindOneAndUpdateOptions::builder()
    .upsert(true)
    .return_document(db::ReturnDocument::After)
    .build();

  let query = db::doc! { "oid": user.sub.clone() };
  let updates = mkplayer(&user);

  let player = collection
    .find_one_and_update(query, updates, options)
    .await
    .map_err(|error| {
      log::warn!("unable to create new player - {:?}", error);
      Error::from_str(500, "player-failure")
    })?
    .ok_or(Error::from_str(404, "missing-player"))?;

  log::info!("found record - {:?}, building token", player);

  let jwt = auth::Claims::for_player(&user.sub, &player.id).encode()?;

  // With our player created, we're ready to store the token in our session and move along.
  let cookie = format!("{}={}; {}", constants::STICKBOT_COOKIE_NAME, jwt, COOKIE_FLAGS);

  let destination = std::env::var(constants::STICKBOT_ONCORE_URL_ENV)
    .ok()
    .unwrap_or("/auth/identify".into());

  // TODO - determine where to send the user. Once the web UI is created, we will send the user to some login page
  // where an attempt will be made to fetch identity information using the newly-set cookie.
  let response = Response::builder(302)
    .header("Set-Cookie", cookie)
    .header("Location", destination.as_str())
    .build();

  Ok(response)
}

// Start the oAuth authentication flow. This is a straightforward redirect to the oAuth provider to queue a log in.
pub async fn start(_: Request) -> Result {
  let client_id = std::env::var(constants::AUTH_O_CLIENT_ID_ENV).ok();
  let auth_uri = std::env::var(constants::AUTH_O_AUTH_URI_ENV).ok();
  let redir_uri = std::env::var(constants::AUTH_O_REDIRECT_URI_ENV).ok();
  log::info!("new user, redirecting to auth flow");

  client_id
    .zip(auth_uri)
    .zip(redir_uri)
    .ok_or(Error::from_str(500, "missing auth creds"))
    .and_then(|((id, auth), redir)| {
      let url = Url::parse_with_params(
        &auth,
        &[
          ("client_id", id.as_str()),
          ("redirect_uri", redir.as_str()),
          ("response_type", &"code"),
          ("scope", &"openid profile email"),
        ],
      )?;
      log::info!("creating auth redir '{}'", &url);
      Ok(Redirect::temporary(url.to_string()).into())
    })
}
