use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CodeBuilder {
  code: Code,
}

impl CodeBuilder {
  pub(crate) fn add_const(&mut self, obj: Object) -> Result<u16> {
    let idx = Self::index(self.code.constants.len(), "constant pool overflow")?;

    self.code.constants.push(obj);

    Ok(idx)
  }

  pub(crate) fn add_local(&mut self, name: &str) -> Result<u16> {
    if let Some(idx) = self.code.locals.iter().position(|n| n == name) {
      return Self::index(idx, "local table overflow");
    }

    let idx = Self::index(self.code.locals.len(), "local table overflow")?;

    self.code.locals.push(name.to_owned());

    Ok(idx)
  }

  pub(crate) fn add_name(&mut self, name: &str) -> Result<u16> {
    if let Some(idx) = self.code.names.iter().position(|n| n == name) {
      return Self::index(idx, "name table overflow");
    }

    let idx = Self::index(self.code.names.len(), "name table overflow")?;

    self.code.names.push(name.to_owned());

    Ok(idx)
  }

  pub(crate) fn current_offset(&self) -> Result<u16> {
    Self::index(self.code.instructions.len(), "instruction offset overflow")
  }

  pub(crate) fn emit(&mut self, instruction: Instruction) {
    self.code.instructions.push(instruction);
  }

  pub(crate) fn emit_jump(&mut self, instruction: Instruction) -> usize {
    let idx = self.code.instructions.len();
    self.emit(instruction);
    idx
  }

  pub(crate) fn finish(self) -> Code {
    self.code
  }

  fn index(idx: usize, message: &str) -> Result<u16> {
    u16::try_from(idx).map_err(|_| Error::Compile {
      message: message.into(),
    })
  }

  pub(crate) fn instructions(&self) -> &[Instruction] {
    &self.code.instructions
  }

  pub(crate) fn patch_jump(&mut self, idx: usize) -> Result {
    let target =
      Self::index(self.code.instructions.len(), "jump target overflow")?;

    let instruction =
      self
        .code
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
