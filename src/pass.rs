use super::*;

mod collect_symbols;
mod emit_bytecode;
mod lower_ast;

pub(crate) use {
  collect_symbols::CollectSymbols, emit_bytecode::EmitBytecode,
  lower_ast::LowerAst,
};

pub(crate) trait Pass {
  fn run(&mut self, context: &mut Context<'_>) -> Result;
}
