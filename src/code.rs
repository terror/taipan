use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Code {
  pub constants: Vec<Object>,
  pub instructions: Vec<Instruction>,
  pub locals: Vec<String>,
  pub names: Vec<String>,
}

impl Code {
  pub(crate) fn add_const(&mut self, obj: Object) -> Result<u16> {
    let idx = Self::index(self.constants.len(), "constant pool overflow")?;

    self.constants.push(obj);

    Ok(idx)
  }

  pub(crate) fn add_local(&mut self, name: &str) -> Result<u16> {
    if let Some(idx) = self.locals.iter().position(|n| n == name) {
      return Self::index(idx, "local table overflow");
    }

    let idx = Self::index(self.locals.len(), "local table overflow")?;

    self.locals.push(name.to_owned());

    Ok(idx)
  }

  pub(crate) fn add_name(&mut self, name: &str) -> Result<u16> {
    if let Some(idx) = self.names.iter().position(|n| n == name) {
      return Self::index(idx, "name table overflow");
    }

    let idx = Self::index(self.names.len(), "name table overflow")?;

    self.names.push(name.to_owned());

    Ok(idx)
  }

  pub(crate) fn current_offset(&self) -> Result<u16> {
    Self::index(self.instructions.len(), "instruction offset overflow")
  }

  pub(crate) fn emit(&mut self, instruction: Instruction) {
    self.instructions.push(instruction);
  }

  pub(crate) fn emit_jump(&mut self, instruction: Instruction) -> usize {
    let idx = self.instructions.len();
    self.emit(instruction);
    idx
  }

  fn index(idx: usize, message: &str) -> Result<u16> {
    u16::try_from(idx).map_err(|_| Error::Compile {
      message: message.into(),
    })
  }

  pub(crate) fn patch_jump(&mut self, idx: usize) -> Result {
    let target = Self::index(self.instructions.len(), "jump target overflow")?;

    let instruction =
      self
        .instructions
        .get_mut(idx)
        .ok_or_else(|| Error::Compile {
          message: "missing jump instruction".into(),
        })?;

    match instruction {
      Instruction::Jump(t)
      | Instruction::PopJumpIfFalse(t)
      | Instruction::PopJumpIfTrue(t) => *t = target,
      _ => {
        return Err(Error::Compile {
          message: "attempted to patch non-jump instruction".into(),
        });
      }
    }

    Ok(())
  }
}
