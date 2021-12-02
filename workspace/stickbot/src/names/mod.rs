use std::io::{BufRead, Result};

const LEFT_DATA: &'static [u8] = include_bytes!("left.txt");
const RIGHT_DATA: &'static [u8] = include_bytes!("right.txt");

pub fn generate() -> Result<String> {
  let mut buf = [0u8; 2];
  getrandom::getrandom(&mut buf)?;
  let (i, j) = (buf[0], buf[1]);

  let left = LEFT_DATA.lines().nth(i as usize);
  let right = RIGHT_DATA.lines().nth(j as usize);

  left
    .zip(right)
    .and_then(|(l, r)| l.ok().zip(r.ok()))
    .map(|(l, r)| format!("{} {}", l, r))
    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "unable to generate name"))
}

#[cfg(test)]
mod test {
  use super::generate;

  #[test]
  fn test_generate() {
    let name = generate().unwrap();
    let bits = name.split(" ").collect::<Vec<&str>>();
    assert_eq!(bits.len(), 2);
  }
}
