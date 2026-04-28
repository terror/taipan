use super::*;

pub(crate) struct Scope {
  pub(crate) code: Code,
  pub(crate) in_function: bool,
}
