use super::*;

mod collect_symbols;
mod emit_bytecode;

pub(crate) use {collect_symbols::CollectSymbols, emit_bytecode::EmitBytecode};

pub(crate) trait Pass {
  fn run(&mut self, context: &mut Context<'_>) -> Result;
}
