use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum NumberKey {
  Float(u64),
  Int(i64),
}

impl NumberKey {
  pub(crate) fn float(value: f64) -> Self {
    if value.is_finite()
      && value.fract() == 0.0
      && let Some(value) = value.to_i64()
    {
      Self::Int(value)
    } else {
      Self::Float(value.to_bits())
    }
  }
}
