use std::io::{Error, ErrorKind, Result};

use chrono;
use jsonwebtoken;
use serde::{Deserialize, Serialize};

use crate::constants;

#[derive(Debug)]
pub enum Authority {
  Player(bankah::PlayerState),
}

impl Authority {
  pub fn player(self) -> Option<bankah::PlayerState> {
    match self {
      Authority::Player(player) => Some(player),
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
  pub exp: usize,
  pub oid: String,
  pub id: String,
}

impl Claims {
  pub fn decode<T>(target: &T) -> Result<Self>
  where
    T: std::fmt::Display,
  {
    let token = format!("{}", target);
    let secret = std::env::var(constants::STICKBOT_JWT_SECRET_ENV).unwrap_or("secret".into());
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    jsonwebtoken::decode::<Self>(token.as_str(), &key, &validation)
      .map_err(|error| {
        log::warn!("unable to decode token - {}", error);
        Error::new(ErrorKind::Other, "bad-jwt")
      })
      .map(|data| data.claims)
  }

  pub fn for_player<T>(oid: T, id: T) -> Self
  where
    T: std::fmt::Display,
  {
    let day = chrono::Utc::now()
      .checked_add_signed(chrono::Duration::minutes(60))
      .unwrap_or(chrono::Utc::now());

    let exp = day.timestamp_millis() as usize;

    Self {
      exp,
      oid: format!("{}", oid),
      id: format!("{}", id),
    }
  }

  pub fn encode(&self) -> Result<String> {
    let header = &jsonwebtoken::Header::default();
    let secret = std::env::var(constants::STICKBOT_JWT_SECRET_ENV).unwrap_or("secret".into());
    let secret = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());

    jsonwebtoken::encode(&header, &self, &secret).map_err(|error| {
      log::warn!("unable to encode token - {}", error);
      Error::new(ErrorKind::Other, "bad-jwt")
    })
  }
}
