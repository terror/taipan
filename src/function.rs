use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Function {
  function: fn(&[Object], &mut dyn Write) -> Result<Object>,
  name: &'static str,
}

impl Function {
  pub(crate) fn call<W: Write>(
    &self,
    arguments: &[Object],
    output: &mut W,
  ) -> Result<Object> {
    (self.function)(arguments, output)
  }

  pub(crate) fn name(&self) -> &'static str {
    self.name
  }

  pub(crate) const fn new(
    name: &'static str,
    function: fn(&[Object], &mut dyn Write) -> Result<Object>,
  ) -> Self {
    Self { function, name }
  }
}
