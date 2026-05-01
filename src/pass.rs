use super::*;

mod collect_symbols;
mod emit_bytecode;
mod validate_syntax;

pub(crate) use {
  collect_symbols::CollectSymbols, emit_bytecode::EmitBytecode,
  validate_syntax::ValidateSyntax,
};

pub(crate) trait Pass {
  fn run(&mut self, context: &mut Context<'_>) -> Result;
}
