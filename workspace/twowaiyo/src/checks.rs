pub fn is_place(amount: u8) -> bool {
  match amount {
    4 | 5 | 6 | 8 | 9 | 10 => true,
    _ => false,
  }
}
