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

  fn bind_arguments(
    parameters: &[String],
    defaults: &[Object],
    arguments: Vec<Object>,
    keywords: Vec<(String, Object)>,
  ) -> Result<Vec<Object>> {
    let argument_count = arguments.len() + keywords.len();

    let required =
      parameters
        .len()
        .checked_sub(defaults.len())
        .ok_or_else(|| Error::Internal {
          message: "invalid default argument count".into(),
        })?;

    if arguments.len() > parameters.len() {
      return Err(Error::TypeError {
        message: format!(
          "expected from {required} to {} arguments, got {argument_count}",
          parameters.len(),
        ),
      });
    }

    let mut bound = vec![None; parameters.len()];

    for (index, argument) in arguments.into_iter().enumerate() {
      bound[index] = Some(argument);
    }

    for (name, value) in keywords {
      let index = parameters
        .iter()
        .position(|parameter| parameter == &name)
        .ok_or_else(|| Error::TypeError {
          message: format!("got unexpected keyword argument '{name}'"),
        })?;

      if bound[index].is_some() {
        return Err(Error::TypeError {
          message: format!("got multiple values for argument '{name}'"),
        });
      }

      bound[index] = Some(value);
    }

    for index in 0..parameters.len() {
      if bound[index].is_none() {
        if index < required {
          return Err(Error::TypeError {
            message: format!(
              "missing required argument '{}'",
              parameters[index]
            ),
          });
        }

        bound[index] = Some(defaults[index - required].clone());
      }
    }

    Ok(bound.into_iter().flatten().collect())
  }

  fn call_function_keywords(
    &mut self,
    positional_count: u8,
    keyword_names: u16,
  ) -> Result {
    let names = self.frame()?.keyword_names(keyword_names)?;

    let keyword_count =
      u8::try_from(names.len()).map_err(|_| Error::Internal {
        message: "invalid keyword argument count".into(),
      })?;

    let values = self.frame_mut()?.pop_arguments(keyword_count)?;

    let keywords = names.into_iter().zip(values).collect();

    self.call_function_with_keywords(positional_count, keywords)
  }

  fn call_function_with_keywords(
    &mut self,
    count: u8,
    keywords: Vec<(String, Object)>,
  ) -> Result {
    let arguments = self.frame_mut()?.pop_arguments(count)?;

    let function = self.frame_mut()?.pop()?;

    match function {
      Object::Function {
        closure,
        defaults,
        name: _,
        parameters: params,
        code,
      } => {
        let arguments =
          Self::bind_arguments(&params, &defaults, arguments, keywords)?;

        self.frames.push(
          Frame::builder()
            .code(code)
            .arguments(arguments)
            .freevars(closure)
            .build()?,
        );
      }
      Object::Builtin(builtin) => {
        if !keywords.is_empty() {
          return Err(Error::TypeError {
            message: "keyword arguments are not supported for builtins".into(),
          });
        }

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
      Instruction::BinarySubscript => {
        self.binary_operation(Object::binary_subscript)?;
      }
      Instruction::BuildList(count) => self.frame_mut()?.build_list(count)?,
      Instruction::BuildString(count) => {
        self.frame_mut()?.build_string(count)?;
      }
      Instruction::BuildTuple(count) => self.frame_mut()?.build_tuple(count)?,
      Instruction::CallFunction(argc) => {
        self.call_function_with_keywords(argc, Vec::new())?;
      }
      Instruction::CallFunctionKeywords {
        keyword_names,
        positional_count,
      } => {
        self.call_function_keywords(positional_count, keyword_names)?;
      }
      Instruction::CompareEq => self.compare_eq()?,
      Instruction::CompareGe => self.binary_operation(Object::compare_ge)?,
      Instruction::CompareGt => self.binary_operation(Object::compare_gt)?,
      Instruction::CompareIn => self.binary_operation(Object::compare_in)?,
      Instruction::CompareLe => self.binary_operation(Object::compare_le)?,
      Instruction::CompareLt => self.binary_operation(Object::compare_lt)?,
      Instruction::CompareNe => self.compare_ne()?,
      Instruction::CompareNotIn => {
        self.binary_operation(Object::compare_not_in)?;
      }
      Instruction::Dup => self.dup()?,
      Instruction::ForIter(target) => self.for_iter(target)?,
      Instruction::GetIter => self.get_iter()?,
      Instruction::Jump(target) => self.jump(target)?,
      Instruction::LoadConst(index) => self.load_const(index)?,
      Instruction::LoadFast(index) => self.load_fast(index)?,
      Instruction::LoadFree(index) => self.load_free(index)?,
      Instruction::LoadName(index) => self.load_name(index)?,
      Instruction::MakeFunction {
        default_count,
        function,
      } => {
        self.make_function(function, default_count)?;
      }
      Instruction::Pop => {
        self.frame_mut()?.pop()?;
      }
      Instruction::PopJumpIfFalse(target) => self.pop_jump_if_false(target)?,
      Instruction::PopJumpIfTrue(target) => self.pop_jump_if_true(target)?,
      Instruction::Return => return self.finish_frame(),
      Instruction::StoreFast(index) => self.store_fast(index)?,
      Instruction::StoreFree(index) => self.store_free(index)?,
      Instruction::StoreName(index) => self.store_name(index)?,
      Instruction::StoreSubscript => self.store_subscript()?,
      Instruction::UnpackSequence(count) => self.unpack_sequence(count)?,
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

  fn for_iter(&mut self, target: u16) -> Result {
    let iterator = self.frame()?.peek()?;

    if let Some(item) = iterator.next()? {
      self.frame_mut()?.push(item);
    } else {
      self.frame_mut()?.pop()?;
      self.frame_mut()?.jump(target)?;
    }

    Ok(())
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

  fn get_iter(&mut self) -> Result {
    let value = self.frame_mut()?.pop()?;

    self.frame_mut()?.push(value.make_iterator()?);

    Ok(())
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

  fn make_function(&mut self, index: u16, default_count: u8) -> Result {
    let defaults = self.frame_mut()?.pop_arguments(default_count)?;

    let function = self.frame()?.load_const(index)?;

    let Object::Function {
      closure: _,
      defaults: _,
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
      defaults,
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

  fn store_subscript(&mut self) -> Result {
    let index = self.frame_mut()?.pop()?;

    let target = self.frame_mut()?.pop()?;

    let value = self.frame_mut()?.pop()?;

    target.store_subscript(&index, value)
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

  fn unpack_sequence(&mut self, count: u16) -> Result {
    let value = self.frame_mut()?.pop()?;

    for element in value.unpack_sequence(usize::from(count))?.into_iter().rev()
    {
      self.frame_mut()?.push(element);
    }

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
