use super::*;

pub(crate) struct Scope {
  pub(crate) code: CodeBuilder,
  pub(crate) in_function: bool,
}
