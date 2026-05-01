use super::*;

pub(crate) struct EmitBytecode;

impl Pass for EmitBytecode {
  fn run(&mut self, context: &mut Context<'_>) -> Result {
    let mut compiler = Compiler::new(context.take_symbols()?);

    compiler.compile_body(context.body())?;

    context.set_code(compiler.finish()?);

    Ok(())
  }
}
