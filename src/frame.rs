use super::*;

#[derive(Default)]
pub(crate) struct Frame {
  arguments: Option<Vec<Object>>,
  code: Option<Rc<Code>>,
  freevars: Vec<Rc<RefCell<Option<Object>>>>,
  ip: usize,
  locals: Vec<Rc<RefCell<Option<Object>>>>,
  stack: Vec<Object>,
}

impl Frame {
  pub(crate) fn arguments(self, arguments: Vec<Object>) -> Self {
    Self {
      arguments: Some(arguments),
      ..self
    }
  }

  pub(crate) fn build(mut self) -> Result<Self> {
    let code = self.code.as_ref().ok_or_else(|| Error::Internal {
      message: "frame missing code".into(),
    })?;

    let arguments = self.arguments.take().unwrap_or_default();

    if arguments.len() > code.locals.len() {
      return Err(Error::Internal {
        message: "invalid argument count".into(),
      });
    }

    self.locals = (0..code.locals.len())
      .map(|_| Rc::new(RefCell::new(None)))
      .collect();

    for (index, argument) in arguments.into_iter().enumerate() {
      *self.locals[index].borrow_mut() = Some(argument);
    }

    Ok(self)
  }

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

  pub(crate) fn capture_cell(
    &self,
    name: &str,
  ) -> Result<Rc<RefCell<Option<Object>>>> {
    if let Some(index) = self
      .code_ref()
      .locals
      .iter()
      .position(|local| local == name)
    {
      return self
        .locals
        .get(index)
        .cloned()
        .ok_or_else(|| Error::Internal {
          message: "invalid local index".into(),
        });
    }

    if let Some(index) = self
      .code_ref()
      .freevars
      .iter()
      .position(|freevar| freevar == name)
    {
      return self.freevars.get(index).cloned().ok_or_else(|| {
        Error::Internal {
          message: "invalid free variable index".into(),
        }
      });
    }

    Err(Error::Internal {
      message: format!("missing closure variable: {name}"),
    })
  }

  pub(crate) fn code(self, code: Rc<Code>) -> Self {
    Self {
      code: Some(code),
      ..self
    }
  }

  fn code_ref(&self) -> &Code {
    self
      .code
      .as_ref()
      .expect("frame should have code after build")
  }

  pub(crate) fn finish(self) -> Object {
    self.stack.into_iter().last().unwrap_or(Object::None)
  }

  fn free_name(&self, index: usize) -> Result<String> {
    self.code_ref().freevars.get(index).cloned().ok_or_else(|| {
      Error::Internal {
        message: "invalid free variable index".into(),
      }
    })
  }

  pub(crate) fn freevars(
    self,
    freevars: Vec<Rc<RefCell<Option<Object>>>>,
  ) -> Self {
    Self { freevars, ..self }
  }

  pub(crate) fn jump(&mut self, target: u16) -> Result {
    let target = usize::from(target);

    if target > self.code_ref().instructions.len() {
      return Err(Error::Internal {
        message: "invalid jump target".into(),
      });
    }

    self.ip = target;

    Ok(())
  }

  pub(crate) fn load_const(&self, index: u16) -> Result<Object> {
    self
      .code_ref()
      .constants
      .get(usize::from(index))
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid constant index".into(),
      })
  }

  pub(crate) fn load_free(&self, index: u16) -> Result<Object> {
    let index = usize::from(index);

    let name = self.free_name(index)?;

    self
      .freevars
      .get(index)
      .ok_or_else(|| Error::Internal {
        message: "invalid free variable index".into(),
      })?
      .borrow()
      .clone()
      .ok_or(Error::UnboundLocal { name })
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
      .borrow()
      .clone()
      .ok_or(Error::UnboundLocal { name })
  }

  fn local_name(&self, index: usize) -> Result<String> {
    self
      .code_ref()
      .locals
      .get(index)
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid local index".into(),
      })
  }

  pub(crate) fn name(&self, index: u16) -> Result<String> {
    self
      .code_ref()
      .names
      .get(usize::from(index))
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid name index".into(),
      })
  }

  pub(crate) fn next_instruction(&mut self) -> Option<Instruction> {
    let instruction = self.code_ref().instructions.get(self.ip).copied()?;

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

  pub(crate) fn store_free(&mut self, index: u16, object: Object) -> Result {
    let index = usize::from(index);

    let cell = self.freevars.get(index).ok_or_else(|| Error::Internal {
      message: "invalid free variable index".into(),
    })?;

    *cell.borrow_mut() = Some(object);

    Ok(())
  }

  pub(crate) fn store_local(&mut self, index: u16, object: Object) -> Result {
    let index = usize::from(index);

    if index >= self.locals.len() {
      return Err(Error::Internal {
        message: "invalid local index".into(),
      });
    }

    *self.locals[index].borrow_mut() = Some(object);

    Ok(())
  }
}
