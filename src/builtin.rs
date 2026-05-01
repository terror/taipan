use super::*;

#[derive(Clone, Copy, Debug)]
pub enum Builtin {
  Function(Function),
}

impl Builtin {
  pub(crate) fn call<W: Write>(
    &self,
    arguments: &[Object],
    output: &mut W,
  ) -> Result<Object> {
    match self {
      Self::Function(function) => function.call(arguments, output),
    }
  }

  #[must_use]
  pub fn name(&self) -> &'static str {
    match self {
      Self::Function(function) => function.name(),
    }
  }
}
