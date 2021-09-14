use std::io::Result;

use twowaiyo::Table;

fn main() -> Result<()> {
  let table = Table::default();
  println!("new table - {:?}", table);
  Ok(())
}
