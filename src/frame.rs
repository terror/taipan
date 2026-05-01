use super::*;

pub(crate) struct Frame {
  code: Rc<Code>,
  freevars: Vec<Cell>,
  ip: usize,
  locals: Vec<Cell>,
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

  pub(crate) fn capture_cell(&self, name: &str) -> Result<Cell> {
    if let Some(index) = self.code.locals.iter().position(|local| local == name)
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
      .code
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

  pub(crate) fn finish(self) -> Object {
    self.stack.into_iter().last().unwrap_or(Object::None)
  }

  fn free_name(&self, index: usize) -> Result<String> {
    self
      .code
      .freevars
      .get(index)
      .cloned()
      .ok_or_else(|| Error::Internal {
        message: "invalid free variable index".into(),
      })
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
    Self::with_closure(code, Vec::new())
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

  pub(crate) fn with_arguments(
    code: Rc<Code>,
    arguments: Vec<Object>,
    freevars: Vec<Cell>,
  ) -> Result<Self> {
    if arguments.len() > code.locals.len() {
      return Err(Error::Internal {
        message: "invalid argument count".into(),
      });
    }

    let frame = Self::with_closure(code, freevars);

    for (index, argument) in arguments.into_iter().enumerate() {
      *frame.locals[index].borrow_mut() = Some(argument);
    }

    Ok(frame)
  }

  pub(crate) fn with_closure(code: Rc<Code>, freevars: Vec<Cell>) -> Self {
    let locals_len = code.locals.len();

    Self {
      code,
      freevars,
      ip: 0,
      locals: (0..locals_len)
        .map(|_| Rc::new(RefCell::new(None)))
        .collect(),
      stack: Vec::new(),
    }
  }
}
