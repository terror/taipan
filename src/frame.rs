use super::*;

pub(crate) struct Frame {
  pub(crate) code: Code,
  pub(crate) ip: usize,
  pub(crate) locals: Vec<Option<Object>>,
  pub(crate) stack: Vec<Object>,
}
