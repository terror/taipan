use super::*;

pub(crate) struct Frame {
  code: Rc<Code>,
  ip: usize,
  locals: Vec<Option<Object>>,
  stack: Vec<Object>,
}

impl Frame {
  pub(crate) fn build_string(&mut self, count: u16) -> Result {
    let count = usize::from(count);

    let start =
      self
        .stack
        .len()
        .checked_sub(count)
        .ok_or_else(|| Error::Internal {
          message: "bytecode stack underflow".into(),
        })?;

    let parts = self.stack[start..]
      .iter()
      .map(ToString::to_string)
      .collect::<String>();

    self.stack.truncate(start);

    self.push(Object::Str(parts));

    Ok(())
  }

  pub(crate) fn finish(self) -> Object {
    self.stack.into_iter().last().unwrap_or(Object::None)
  }

  pub(crate) fn jump(&mut self, target: u16) -> Result {
    let target = usize::from(target);

    if target > self.code.instructions.len() {
      return Err(Error::Internal {
        message: "invalid jump target".into(),
      });
    }

    self.ip = target;

    Ok(())
  }

  pub(crate) fn load_const(&self, index: u16) -> Result<Object> {
    self
      .code
      .constants
      .get(usize::from(index))
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid constant index".into(),
      })
  }

  pub(crate) fn load_local(&self, index: u16) -> Result<Object> {
    let index = usize::from(index);

    let name = self.local_name(index)?;

    self
      .locals
      .get(index)
      .ok_or_else(|| Error::Internal {
        message: "invalid local index".into(),
      })?
      .clone()
      .ok_or(Error::UnboundLocal { name })
  }

  fn local_name(&self, index: usize) -> Result<String> {
    self
      .code
      .locals
      .get(index)
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid local index".into(),
      })
  }

  pub(crate) fn name(&self, index: u16) -> Result<String> {
    self
      .code
      .names
      .get(usize::from(index))
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid name index".into(),
      })
  }

  pub(crate) fn new(code: Rc<Code>) -> Self {
    let locals_len = code.locals.len();

    Self {
      code,
      ip: 0,
      locals: vec![None; locals_len],
      stack: Vec::new(),
    }
  }

  pub(crate) fn next_instruction(&mut self) -> Option<Instruction> {
    let instruction = self.code.instructions.get(self.ip).copied()?;

    self.ip += 1;

    Some(instruction)
  }

  pub(crate) fn peek(&self) -> Result<Object> {
    self.stack.last().cloned().ok_or_else(|| Error::Internal {
      message: "bytecode stack underflow".into(),
    })
  }

  pub(crate) fn pop(&mut self) -> Result<Object> {
    self.stack.pop().ok_or_else(|| Error::Internal {
      message: "bytecode stack underflow".into(),
    })
  }

  pub(crate) fn pop2(&mut self) -> Result<(Object, Object)> {
    let rhs = self.pop()?;
    let lhs = self.pop()?;
    Ok((lhs, rhs))
  }

  pub(crate) fn pop_arguments(&mut self, count: u8) -> Result<Vec<Object>> {
    Ok(
      self.stack.split_off(
        self
          .stack
          .len()
          .checked_sub(usize::from(count))
          .ok_or_else(|| Error::Internal {
            message: "bytecode stack underflow".into(),
          })?,
      ),
    )
  }

  pub(crate) fn push(&mut self, object: Object) {
    self.stack.push(object);
  }

  pub(crate) fn store_local(&mut self, index: u16, object: Object) -> Result {
    let index = usize::from(index);

    if index >= self.locals.len() {
      return Err(Error::Internal {
        message: "invalid local index".into(),
      });
    }

    self.locals[index] = Some(object);

    Ok(())
  }

  pub(crate) fn with_arguments(
    code: Rc<Code>,
    arguments: Vec<Object>,
  ) -> Result<Self> {
    if arguments.len() > code.locals.len() {
      return Err(Error::Internal {
        message: "invalid argument count".into(),
      });
    }

    let mut frame = Self::new(code);

    for (index, argument) in arguments.into_iter().enumerate() {
      frame.locals[index] = Some(argument);
    }

    Ok(frame)
  }
}
