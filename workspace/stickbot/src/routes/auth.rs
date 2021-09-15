use serde::{Deserialize, Serialize};
use surf;

use crate::auth::from_token as load_user_info;
use crate::constants;
use crate::web::{Body, Cookie, Error, Redirect, Request, Response, Result, Url};

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

pub async fn identify(request: Request) -> Result {
  let cookie = request
    .header("Cookie")
    .and_then(|list| list.get(0))
    .map(|value| value.to_string())
    .and_then(|cook| Cookie::parse(cook).ok())
    .ok_or(Error::from_str(404, ""))?;

  log::info!("found cookie header, loading user info");

  load_user_info(cookie.value())
    .await
    .and_then(|parsed| Body::from_json(&parsed).ok())
    .map(|body| Response::builder(200).body(body).build())
    .ok_or(Error::from_str(404, ""))
}

pub async fn complete(request: Request) -> Result {
  log::info!("completing auth flow");
  let code = request
    .url()
    .query_pairs()
    .find_map(|(k, v)| if k == "code" { Some(v) } else { None })
    .ok_or(Error::from_str(404, ""))?;

  let payload = AuthCodeRequest {
    code: code.into(),
    ..AuthCodeRequest::default()
  };

  let destination = std::env::var(constants::AUTH_O_TOKEN_URI_ENV).unwrap_or_default();

  log::info!("exchanging code - {} (at {})", payload.code, destination);
  let mut response = surf::post(&destination).body_json(&payload)?.await?;
  let token = token_from_response(&mut response).await;

  token.map_or(Ok(Redirect::temporary("/heartbeat").into()), |token| {
    log::info!("created token, sending to cookie storage");
    let cookie = format!("{}={}; {}", constants::STICKBOT_COOKIE_NAME, token, COOKIE_FLAGS);

    // TODO - determine where to send the user.
    let response = Response::builder(302)
      .header("Set-Cookie", cookie)
      .header("Location", "/auth/identify")
      .build();

    Ok(response)
  })
}

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
