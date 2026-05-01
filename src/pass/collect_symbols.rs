use super::*;

pub(crate) struct CollectSymbols;

impl Pass for CollectSymbols {
  fn run(&mut self, context: &mut Context<'_>) -> Result {
    context.set_symbols(SymbolTable::module(&context.ast()?.body)?);

    Ok(())
  }
}
