use super::*;

#[derive(Clone, Copy, Debug)]
pub enum Builtin {
  Function {
    function: fn(&[Object], &mut Heap, &mut dyn Write) -> Result<Object>,
    name: &'static str,
  },
}

impl Builtin {
  pub(crate) fn call<W: Write>(
    &self,
    arguments: &[Object],
    heap: &mut Heap,
    output: &mut W,
  ) -> Result<Object> {
    match self {
      Self::Function { function, name: _ } => function(arguments, heap, output),
    }
  }

  #[must_use]
  pub fn name(&self) -> &'static str {
    match self {
      Self::Function { function: _, name } => name,
    }
  }
}
