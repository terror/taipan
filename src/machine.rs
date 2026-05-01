use super::*;

pub struct Machine<W: Write> {
  frames: Vec<Frame>,
  globals: HashMap<String, Object>,
  output: W,
}

impl Machine<Stdout> {
  /// Runs `code` with standard output.
  ///
  /// # Errors
  ///
  /// Returns an error if execution fails.
  pub fn run(code: Code) -> Result<Object> {
    let mut machine = Machine {
      frames: Vec::new(),
      globals: HashMap::new(),
      output: io::stdout(),
    };

    machine.initialize();

    machine.execute(code)
  }
}

impl<W: Write> Machine<W> {
  fn binary_operation(
    &mut self,
    operation: fn(&Object, &Object) -> Result<Object>,
  ) -> Result {
    let (lhs, rhs) = self.frame_mut()?.pop2()?;

    self.frame_mut()?.push(operation(&lhs, &rhs)?);

    Ok(())
  }

  fn build_string(&mut self, count: u16) -> Result {
    self.frame_mut()?.build_string(count)
  }

  fn call_function(&mut self, count: u8) -> Result {
    let argument_count = usize::from(count);

    let arguments = self.frame_mut()?.pop_arguments(count)?;

    let function = self.frame_mut()?.pop()?;

    match function {
      Object::Function {
        closure,
        name: _,
        parameters: params,
        code,
      } => {
        if params.len() != argument_count {
          return Err(Error::TypeError {
            message: format!(
              "expected {} arguments, got {argument_count}",
              params.len()
            ),
          });
        }

        self.frames.push(
          Frame::builder()
            .code(code)
            .arguments(arguments)
            .freevars(closure)
            .build()?,
        );
      }
      Object::Builtin(builtin) => {
        let result = builtin.call(&arguments, &mut self.output)?;
        self.frame_mut()?.push(result);
      }
      _ => {
        return Err(Error::TypeError {
          message: format!("'{}' object is not callable", function.type_name()),
        });
      }
    }

    Ok(())
  }

  fn compare_eq(&mut self) -> Result {
    let (lhs, rhs) = self.frame_mut()?.pop2()?;
    self.frame_mut()?.push(lhs.compare_eq(&rhs));
    Ok(())
  }

  fn compare_ne(&mut self) -> Result {
    let (lhs, rhs) = self.frame_mut()?.pop2()?;
    self.frame_mut()?.push(lhs.compare_ne(&rhs));
    Ok(())
  }

  fn dup(&mut self) -> Result {
    let value = self.frame()?.peek()?;
    self.frame_mut()?.push(value);
    Ok(())
  }

  fn execute(&mut self, code: Code) -> Result<Object> {
    self
      .frames
      .push(Frame::builder().code(Rc::new(code)).build()?);

    self.run_loop()
  }

  fn execute_instruction(
    &mut self,
    instruction: Instruction,
  ) -> Result<Option<Object>> {
    match instruction {
      Instruction::BinaryAdd => self.binary_operation(Object::binary_add)?,
      Instruction::BinaryBitAnd => {
        self.binary_operation(Object::binary_bit_and)?;
      }
      Instruction::BinaryBitOr => {
        self.binary_operation(Object::binary_bit_or)?;
      }
      Instruction::BinaryBitXor => {
        self.binary_operation(Object::binary_bit_xor)?;
      }
      Instruction::BinaryDiv => self.binary_operation(Object::binary_div)?,
      Instruction::BinaryFloorDiv => {
        self.binary_operation(Object::binary_floor_div)?;
      }
      Instruction::BinaryLShift => {
        self.binary_operation(Object::binary_lshift)?;
      }
      Instruction::BinaryMod => self.binary_operation(Object::binary_mod)?,
      Instruction::BinaryMul => self.binary_operation(Object::binary_mul)?,
      Instruction::BinaryPow => self.binary_operation(Object::binary_pow)?,
      Instruction::BinaryRShift => {
        self.binary_operation(Object::binary_rshift)?;
      }
      Instruction::BinarySub => self.binary_operation(Object::binary_sub)?,
      Instruction::BuildString(count) => self.build_string(count)?,
      Instruction::CallFunction(argc) => self.call_function(argc)?,
      Instruction::CompareEq => self.compare_eq()?,
      Instruction::CompareGe => self.binary_operation(Object::compare_ge)?,
      Instruction::CompareGt => self.binary_operation(Object::compare_gt)?,
      Instruction::CompareLe => self.binary_operation(Object::compare_le)?,
      Instruction::CompareLt => self.binary_operation(Object::compare_lt)?,
      Instruction::CompareNe => self.compare_ne()?,
      Instruction::Dup => self.dup()?,
      Instruction::Jump(target) => self.jump(target)?,
      Instruction::LoadConst(index) => self.load_const(index)?,
      Instruction::LoadFast(index) => self.load_fast(index)?,
      Instruction::LoadFree(index) => self.load_free(index)?,
      Instruction::LoadName(index) => self.load_name(index)?,
      Instruction::MakeFunction(index) => self.make_function(index)?,
      Instruction::Pop => {
        self.frame_mut()?.pop()?;
      }
      Instruction::PopJumpIfFalse(target) => self.pop_jump_if_false(target)?,
      Instruction::PopJumpIfTrue(target) => self.pop_jump_if_true(target)?,
      Instruction::Return => return self.finish_frame(),
      Instruction::StoreFast(index) => self.store_fast(index)?,
      Instruction::StoreFree(index) => self.store_free(index)?,
      Instruction::StoreName(index) => self.store_name(index)?,
      Instruction::UnaryInvert => self.unary_invert()?,
      Instruction::UnaryNeg => self.unary_neg()?,
      Instruction::UnaryNot => self.unary_not()?,
      Instruction::UnaryPos => self.unary_pos()?,
    }

    Ok(None)
  }

  fn finish_frame(&mut self) -> Result<Option<Object>> {
    let frame = self.frames.pop().ok_or_else(|| Error::Internal {
      message: "missing frame".into(),
    })?;

    let ret = frame.finish();

    if self.frames.is_empty() {
      return Ok(Some(ret));
    }

    self.frame_mut()?.push(ret);

    Ok(None)
  }

  fn frame(&self) -> Result<&Frame> {
    self.frames.last().ok_or_else(|| Error::Internal {
      message: "missing frame".into(),
    })
  }

  fn frame_mut(&mut self) -> Result<&mut Frame> {
    self.frames.last_mut().ok_or_else(|| Error::Internal {
      message: "missing frame".into(),
    })
  }

  fn initialize(&mut self) {
    for builtin in BUILTINS {
      self
        .globals
        .insert(builtin.name().into(), Object::Builtin(*builtin));
    }
  }

  fn jump(&mut self, target: u16) -> Result {
    self.frame_mut()?.jump(target)
  }

  fn load_const(&mut self, index: u16) -> Result {
    let value = self.frame()?.load_const(index)?;
    self.frame_mut()?.push(value);
    Ok(())
  }

  fn load_fast(&mut self, index: u16) -> Result {
    let value = self.frame()?.load_local(index)?;
    self.frame_mut()?.push(value);
    Ok(())
  }

  fn load_free(&mut self, index: u16) -> Result {
    let value = self.frame()?.load_free(index)?;
    self.frame_mut()?.push(value);
    Ok(())
  }

  fn load_name(&mut self, index: u16) -> Result {
    let name = self.frame()?.name(index)?;

    let value = self
      .globals
      .get(&name)
      .ok_or(Error::NameError { name })?
      .clone();

    self.frame_mut()?.push(value);

    Ok(())
  }

  fn make_function(&mut self, index: u16) -> Result {
    let function = self.frame()?.load_const(index)?;

    let Object::Function {
      closure: _,
      name,
      parameters,
      code,
    } = function
    else {
      return Err(Error::Internal {
        message: "invalid function constant".into(),
      });
    };

    let closure = code
      .freevars
      .iter()
      .map(|name| self.frame()?.capture_cell(name))
      .collect::<Result<Vec<_>>>()?;

    self.frame_mut()?.push(Object::Function {
      closure,
      name,
      parameters,
      code,
    });

    Ok(())
  }

  fn next_instruction(&mut self) -> Result<Option<Instruction>> {
    Ok(self.frame_mut()?.next_instruction())
  }

  fn pop_jump_if_false(&mut self, target: u16) -> Result {
    let object = self.frame_mut()?.pop()?;

    if !object.is_truthy() {
      self.frame_mut()?.jump(target)?;
    }

    Ok(())
  }

  fn pop_jump_if_true(&mut self, target: u16) -> Result {
    let object = self.frame_mut()?.pop()?;

    if object.is_truthy() {
      self.frame_mut()?.jump(target)?;
    }

    Ok(())
  }

  fn run_loop(&mut self) -> Result<Object> {
    loop {
      if let Some(result) = self.step()? {
        return Ok(result);
      }
    }
  }

  fn step(&mut self) -> Result<Option<Object>> {
    let Some(instruction) = self.next_instruction()? else {
      return self.finish_frame();
    };

    self.execute_instruction(instruction)
  }

  fn store_fast(&mut self, index: u16) -> Result {
    let value = self.frame_mut()?.pop()?;
    self.frame_mut()?.store_local(index, value)
  }

  fn store_free(&mut self, index: u16) -> Result {
    let value = self.frame_mut()?.pop()?;
    self.frame_mut()?.store_free(index, value)
  }

  fn store_name(&mut self, index: u16) -> Result {
    let name = self.frame()?.name(index)?;

    let value = self.frame_mut()?.pop()?;

    self.globals.insert(name, value);

    Ok(())
  }

  fn unary_invert(&mut self) -> Result {
    let value = self.frame_mut()?.pop()?;

    self.frame_mut()?.push(value.unary_invert()?);

    Ok(())
  }

  fn unary_neg(&mut self) -> Result {
    let value = self.frame_mut()?.pop()?;

    self.frame_mut()?.push(value.unary_neg()?);

    Ok(())
  }

  fn unary_not(&mut self) -> Result {
    let value = self.frame_mut()?.pop()?;

    self.frame_mut()?.push(value.unary_not());

    Ok(())
  }

  fn unary_pos(&mut self) -> Result {
    let value = self.frame_mut()?.pop()?;

    self.frame_mut()?.push(value.unary_pos()?);

    Ok(())
  }

  /// Runs `code` with `output`.
  ///
  /// # Errors
  ///
  /// Returns an error if execution fails or writing to `output` fails.
  pub fn with_output(code: Code, output: W) -> Result<(Object, W)> {
    let mut machine = Machine {
      frames: Vec::new(),
      globals: HashMap::new(),
      output,
    };

    machine.initialize();

    let result = machine.execute(code)?;

    Ok((result, machine.output))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn invalid_bytecode_errors() {
    #[track_caller]
    fn case(code: Code, expected: &str) {
      let result = Machine::with_output(code, Vec::new());

      assert!(
        result.unwrap_err().to_string().contains(expected),
        "expected error to contain: `{expected}`",
      );
    }

    case(
      Code {
        instructions: vec![Instruction::BinaryAdd],
        ..Default::default()
      },
      "bytecode stack underflow",
    );

    case(
      Code {
        instructions: vec![Instruction::LoadConst(0)],
        ..Default::default()
      },
      "invalid constant index",
    );

    case(
      Code {
        instructions: vec![Instruction::LoadFast(0)],
        ..Default::default()
      },
      "invalid local index",
    );

    case(
      Code {
        instructions: vec![Instruction::LoadName(0)],
        ..Default::default()
      },
      "invalid name index",
    );

    case(
      Code {
        instructions: vec![Instruction::Jump(2)],
        ..Default::default()
      },
      "invalid jump target",
    );
  }
}
