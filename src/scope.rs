use super::*;

pub(crate) struct Scope {
  pub(crate) code: CodeBuilder,
  pub(crate) symbols: SymbolTable,
}
