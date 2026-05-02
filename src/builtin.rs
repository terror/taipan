use super::*;

#[derive(Clone, Copy, Debug)]
pub enum Builtin {
  Function {
    function: fn(&[Object], &mut dyn Write) -> Result<Object>,
    name: &'static str,
  },
}

impl Builtin {
  pub(crate) fn call<W: Write>(
    &self,
    arguments: &[Object],
    output: &mut W,
  ) -> Result<Object> {
    match self {
      Self::Function { function, name: _ } => function(arguments, output),
    }
  }

  #[must_use]
  pub fn name(&self) -> &'static str {
    match self {
      Self::Function { function: _, name } => name,
    }
  }
}
