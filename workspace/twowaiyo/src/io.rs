#[derive(Debug)]
pub enum Action {
  Exit,
  Roll,
}

impl Action {
  pub fn parse<T>(input: T) -> Option<Self>
  where
    T: std::fmt::Display,
  {
    let nice = format!("{}", input);

    match nice.as_str() {
      "" => Some(Action::Roll),
      "exit" => Some(Action::Exit),
      "roll" => Some(Action::Roll),
      _ => None,
    }
  }
}
