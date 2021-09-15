use tide;

pub use tide::{Body, Error, Response, Result};
pub type Request = tide::Request<crate::Services>;
