use super::*;

pub(crate) struct Pipeline<'a> {
  context: Context<'a>,
  passes: Vec<Box<dyn Pass>>,
}

impl<'a> Pipeline<'a> {
  fn add_pass(&mut self, pass: Box<dyn Pass>) {
    self.passes.push(pass);
  }

  pub(crate) fn new(context: Context<'a>) -> Self {
    Self {
      context,
      passes: Vec::new(),
    }
  }

  pub(crate) fn run(mut self) -> Result<Context<'a>> {
    for pass in &mut self.passes {
      pass.run(&mut self.context)?;
    }

    Ok(self.context)
  }

  pub(crate) fn with_default_passes(context: Context<'a>) -> Self {
    let mut pipeline = Self::new(context);

    let passes: Vec<Box<dyn Pass>> =
      vec![Box::new(CollectSymbols), Box::new(EmitBytecode)];

    for pass in passes {
      pipeline.add_pass(pass);
    }

    pipeline
  }
}
