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
        name: _,
        params,
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

        self.frames.push(Frame::with_arguments(code, arguments)?);
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
    self.frames.push(Frame::new(code));
    self.run_loop()
  }

  fn execute_instruction(
    &mut self,
    instruction: Instruction,
  ) -> Result<Option<Object>> {
    match instruction {
      Instruction::BinaryAdd => self.binary_operation(Object::binary_add)?,
      Instruction::BinaryDiv => self.binary_operation(Object::binary_div)?,
      Instruction::BinaryFloorDiv => {
        self.binary_operation(Object::binary_floor_div)?;
      }
      Instruction::BinaryMod => self.binary_operation(Object::binary_mod)?,
      Instruction::BinaryMul => self.binary_operation(Object::binary_mul)?,
      Instruction::BinaryPow => self.binary_operation(Object::binary_pow)?,
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
      Instruction::LoadConst(index) | Instruction::MakeFunction(index) => {
        self.load_const(index)?;
      }
      Instruction::LoadFast(index) => self.load_fast(index)?,
      Instruction::LoadName(index) => self.load_name(index)?,
      Instruction::Pop => {
        self.frame_mut()?.pop()?;
      }
      Instruction::PopJumpIfFalse(target) => self.pop_jump_if_false(target)?,
      Instruction::PopJumpIfTrue(target) => self.pop_jump_if_true(target)?,
      Instruction::Return => return self.finish_frame(),
      Instruction::StoreFast(index) => self.store_fast(index)?,
      Instruction::StoreName(index) => self.store_name(index)?,
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

  fn store_name(&mut self, index: u16) -> Result {
    let name = self.frame()?.name(index)?;

    let value = self.frame_mut()?.pop()?;

    self.globals.insert(name, value);

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

  #[cfg(test)]
  pub(crate) fn with_output(code: Code, output: W) -> Result<(Object, W)> {
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
  use {
    super::*,
    indoc::indoc,
    pretty_assertions::assert_eq,
    ruff_python_parser::{Mode, parse},
  };

  #[derive(Debug)]
  struct Test {
    expected_error: Option<&'static str>,
    expected_output: &'static str,
    expected_result: Object,
    source: &'static str,
  }

  impl Test {
    fn expected_error(self, expected_error: &'static str) -> Self {
      Self {
        expected_error: Some(expected_error),
        ..self
      }
    }

    fn expected_output(self, expected_output: &'static str) -> Self {
      Self {
        expected_output,
        ..self
      }
    }

    fn new(source: &'static str) -> Self {
      Self {
        expected_error: None,
        expected_output: "",
        expected_result: Object::None,
        source,
      }
    }

    fn run(self) {
      let output = Vec::new();

      let module = parse(self.source, Mode::Module.into())
        .unwrap()
        .try_into_module()
        .unwrap();

      let result = Machine::with_output(
        Compiler::compile(module.syntax()).unwrap(),
        output,
      );

      if let Some(expected_error) = self.expected_error {
        assert!(
          result.unwrap_err().to_string().contains(expected_error),
          "expected error to contain: `{expected_error}`",
        );

        return;
      }

      let (result, output) = result.unwrap();

      assert_eq!(result, self.expected_result);
      assert_eq!(String::from_utf8(output).unwrap(), self.expected_output);
    }
  }

  #[test]
  fn arithmetic() {
    Test::new("print(1 + 2)\n").expected_output("3\n").run();
  }

  #[test]
  fn aug_assign() {
    Test::new(indoc! {
      "
      foo = 10
      foo += 5
      print(foo)
      "
    })
    .expected_output("15\n")
    .run();
  }

  #[test]
  fn bool_ops() {
    Test::new("print(1 and 2)\n").expected_output("2\n").run();
    Test::new("print(0 and 2)\n").expected_output("0\n").run();
    Test::new("print(1 or 2)\n").expected_output("1\n").run();
    Test::new("print(0 or 2)\n").expected_output("2\n").run();
  }

  #[test]
  fn comparison_ops() {
    Test::new("print(1 < 2)\n").expected_output("True\n").run();
    Test::new("print(2 < 1)\n").expected_output("False\n").run();
    Test::new("print(1 == 1)\n").expected_output("True\n").run();
    Test::new("print(1 != 2)\n").expected_output("True\n").run();
  }

  #[test]
  fn function_call() {
    Test::new(indoc! {
      "
      def foo(bar):
        return bar + 1
      print(foo(41))
      "
    })
    .expected_output("42\n")
    .run();
  }

  #[test]
  fn greet_example() {
    Test::new(indoc! {
      r#"
      x = 1 + 2
      print(x)

      def greet(name):
        return "Hello, " + name

      print(greet("world"))
      "#
    })
    .expected_output("3\nHello, world\n")
    .run();
  }

  #[test]
  fn if_elif_else() {
    Test::new(indoc! {
      r#"
      if 1:
        print("foo")
      elif 1:
        print("bar")
      else:
        print("baz")
      "#
    })
    .expected_output("foo\n")
    .run();

    Test::new(indoc! {
      r#"
      if 0:
        print("foo")
      elif 1:
        print("bar")
      else:
        print("baz")
      "#
    })
    .expected_output("bar\n")
    .run();

    Test::new(indoc! {
      r#"
      if 0:
        print("foo")
      elif 0:
        print("bar")
      else:
        print("baz")
      "#
    })
    .expected_output("baz\n")
    .run();
  }

  #[test]
  fn if_else() {
    Test::new(indoc! {
      r#"
      if 0:
        print("foo")
      else:
        print("bar")
      "#
    })
    .expected_output("bar\n")
    .run();
  }

  #[test]
  fn if_false_branch() {
    Test::new(indoc! {
      r#"
      if 0:
        print("foo")
      "#
    })
    .expected_output("")
    .run();
  }

  #[test]
  fn if_true_branch() {
    Test::new(indoc! {
      r#"
      if 1:
        print("foo")
      "#
    })
    .expected_output("foo\n")
    .run();
  }

  #[test]
  fn implicit_return() {
    Test::new(indoc! {
      "
      def foo():
        pass
      print(foo())
      "
    })
    .expected_output("None\n")
    .run();
  }

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

  #[test]
  fn multiple_args() {
    Test::new("print(\"foo\", \"bar\", \"baz\")\n")
      .expected_output("foo bar baz\n")
      .run();
  }

  #[test]
  fn name_error() {
    Test::new("foo\n").expected_error("foo").run();
  }

  #[test]
  fn nested_function() {
    Test::new(indoc! {
      "
      def foo(bar):
        def baz(qux):
          return qux * 2
        return baz(bar) + 1
      print(foo(5))
      "
    })
    .expected_output("11\n")
    .run();
  }

  #[test]
  fn string_concatenation() {
    Test::new("print(\"foo\" + \"bar\")\n")
      .expected_output("foobar\n")
      .run();
  }

  #[test]
  fn ternary() {
    Test::new("print(\"foo\" if 1 else \"bar\")\n")
      .expected_output("foo\n")
      .run();

    Test::new("print(\"foo\" if 0 else \"bar\")\n")
      .expected_output("bar\n")
      .run();
  }

  #[test]
  fn variable_assignment() {
    Test::new("foo = 42\nprint(foo)\n")
      .expected_output("42\n")
      .run();
  }

  #[test]
  fn while_loop() {
    Test::new(indoc! {
      "
      foo = 0
      while foo < 3:
        print(foo)
        foo += 1
      "
    })
    .expected_output("0\n1\n2\n")
    .run();
  }
}
