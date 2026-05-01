use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CodeBuilder {
  code: Code,
  labels: Vec<Option<u16>>,
  patches: Vec<Vec<usize>>,
}

impl CodeBuilder {
  pub(crate) fn add_const(&mut self, object: Object) -> Result<u16> {
    let index =
      Self::index(self.code.constants.len(), "constant pool overflow")?;

    self.code.constants.push(object);

    Ok(index)
  }

  pub(crate) fn add_freevar(&mut self, name: &str) -> Result<u16> {
    if let Some(index) = self.code.freevars.iter().position(|n| n == name) {
      return Self::index(index, "free variable table overflow");
    }

    let index =
      Self::index(self.code.freevars.len(), "free variable table overflow")?;

    self.code.freevars.push(name.to_owned());

    Ok(index)
  }

  pub(crate) fn add_local(&mut self, name: &str) -> Result<u16> {
    if let Some(index) = self.code.locals.iter().position(|n| n == name) {
      return Self::index(index, "local table overflow");
    }

    let index = Self::index(self.code.locals.len(), "local table overflow")?;

    self.code.locals.push(name.to_owned());

    Ok(index)
  }

  pub(crate) fn add_name(&mut self, name: &str) -> Result<u16> {
    if let Some(index) = self.code.names.iter().position(|n| n == name) {
      return Self::index(index, "name table overflow");
    }

    let index = Self::index(self.code.names.len(), "name table overflow")?;

    self.code.names.push(name.to_owned());

    Ok(index)
  }

  pub(crate) fn current_offset(&self) -> Result<u16> {
    Self::index(self.code.instructions.len(), "instruction offset overflow")
  }

  pub(crate) fn emit(&mut self, instruction: Instruction) {
    self.code.instructions.push(instruction);
  }

  pub(crate) fn emit_jump(&mut self, label: usize) -> Result {
    self.emit_labeled_jump(label, Instruction::Jump)
  }

  pub(crate) fn emit_jump_if_false(&mut self, label: usize) -> Result {
    self.emit_labeled_jump(label, Instruction::PopJumpIfFalse)
  }

  pub(crate) fn emit_jump_if_true(&mut self, label: usize) -> Result {
    self.emit_labeled_jump(label, Instruction::PopJumpIfTrue)
  }

  fn emit_labeled_jump(
    &mut self,
    label: usize,
    instruction: fn(u16) -> Instruction,
  ) -> Result {
    let target =
      self
        .labels
        .get(label)
        .copied()
        .ok_or_else(|| Error::Compile {
          message: "missing label".into(),
        })?;

    let unresolved = target.is_none();

    let target = target.unwrap_or_default();

    let index = self.code.instructions.len();

    self.emit(instruction(target));

    if unresolved {
      let patches =
        self.patches.get_mut(label).ok_or_else(|| Error::Compile {
          message: "missing label patches".into(),
        })?;

      patches.push(index);
    }

    Ok(())
  }

  pub(crate) fn finish(self) -> Result<Code> {
    if self.labels.iter().any(Option::is_none) {
      return Err(Error::Compile {
        message: "unmarked label".into(),
      });
    }

    Ok(self.code)
  }

  fn index(index: usize, message: &str) -> Result<u16> {
    u16::try_from(index).map_err(|_| Error::Compile {
      message: message.into(),
    })
  }

  pub(crate) fn instructions(&self) -> &[Instruction] {
    &self.code.instructions
  }

  pub(crate) fn label(&mut self) -> usize {
    let label = self.labels.len();

    self.labels.push(None);
    self.patches.push(Vec::new());

    label
  }

  pub(crate) fn mark(&mut self, label: usize) -> Result {
    let target = self.current_offset()?;

    let marked = self.labels.get_mut(label).ok_or_else(|| Error::Compile {
      message: "missing label".into(),
    })?;

    if marked.is_some() {
      return Err(Error::Compile {
        message: "label already marked".into(),
      });
    }

    *marked = Some(target);

    let patches = self.patches.get(label).ok_or_else(|| Error::Compile {
      message: "missing label patches".into(),
    })?;

    for index in patches.clone() {
      self.patch_jump(index, target)?;
    }

    Ok(())
  }

  fn patch_jump(&mut self, index: usize, target: u16) -> Result {
    let instruction =
      self
        .code
        .instructions
        .get_mut(index)
        .ok_or_else(|| Error::Compile {
          message: "missing jump instruction".into(),
        })?;

    match instruction {
      Instruction::Jump(jump_target)
      | Instruction::PopJumpIfFalse(jump_target)
      | Instruction::PopJumpIfTrue(jump_target) => *jump_target = target,
      _ => {
        return Err(Error::Compile {
          message: "attempted to patch non-jump instruction".into(),
        });
      }
    }

    Ok(())
  }
}
