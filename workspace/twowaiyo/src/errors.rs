pub use std::io::{Error, ErrorKind};

pub fn mkerr(reason: &str) -> Error {
  Error::new(ErrorKind::Other, reason)
}

pub struct CarryError<T> {
  kind: T,
  error: Error,
}

impl<T> CarryError<T> {
  pub fn new(item: T, reason: &str) -> CarryError<T> {
    CarryError {
      error: mkerr(reason),
      kind: item,
    }
  }

  pub fn consume(self) -> T {
    self.kind
  }
}

impl<T> std::fmt::Debug for CarryError<T> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "{}", self.error)
  }
}
