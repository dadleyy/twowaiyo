#[derive(Default, Clone)]
pub struct RandomRoller {}

impl Iterator for RandomRoller {
  type Item = u8;

  fn next(&mut self) -> Option<Self::Item> {
    let mut buffer = [0u8, 1];

    getrandom::getrandom(&mut buffer)
      .ok()
      .and_then(|_| buffer.iter().next())
      .map(|value| value.rem_euclid(6) + 1)
  }
}
