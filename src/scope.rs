use super::*;

pub(crate) struct Scope {
  pub(crate) code: CodeBuilder,
  pub(crate) loops: Vec<(usize, usize)>,
  pub(crate) symbols: SymbolTable,
}
