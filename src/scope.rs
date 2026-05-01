use super::*;

pub(crate) struct Scope {
  pub(crate) code: CodeBuilder,
  pub(crate) loops: Vec<(Label, Label)>,
  pub(crate) symbols: SymbolTable,
}
