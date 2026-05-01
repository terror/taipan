use super::*;

pub(crate) struct Scope {
  pub(crate) code: CodeBuilder,
  pub(crate) control_flows: Vec<ControlFlow>,
  pub(crate) symbols: SymbolTable,
}
