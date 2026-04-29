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
  fn binary_op(
    &mut self,
    op: fn(&Object, &Object) -> Result<Object>,
  ) -> Result<()> {
    let frame = self.frames.last_mut().unwrap();

    let rhs = frame.stack.pop().unwrap();
    let lhs = frame.stack.pop().unwrap();

    let result = op(&lhs, &rhs)?;

    self.frames.last_mut().unwrap().stack.push(result);

    Ok(())
  }

  fn build_string(&mut self, count: u16) {
    let frame = self.frames.last_mut().unwrap();

    let start = frame.stack.len() - usize::from(count);

    let parts = frame.stack[start..]
      .iter()
      .map(ToString::to_string)
      .collect::<String>();

    frame.stack.truncate(start);
    frame.stack.push(Object::Str(parts));
  }

  fn call_function(&mut self, count: u8) -> Result<()> {
    let argument_count = usize::from(count);

    let frame = self.frames.last_mut().unwrap();

    let arguments = frame.stack.split_off(frame.stack.len() - argument_count);

    let function = frame.stack.pop().unwrap();

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

        let mut locals = vec![None; code.locals.len()];

        for (index, argument) in arguments.into_iter().enumerate() {
          locals[index] = Some(argument);
        }

        self.frames.push(Frame {
          code,
          ip: 0,
          locals,
          stack: Vec::new(),
        });
      }
      Object::BuiltinFn(bf) => {
        if bf.name == "print" {
          write_print(&arguments, &mut self.output)?;
          self.frames.last_mut().unwrap().stack.push(Object::None);
        } else {
          let result = (bf.func)(&arguments)?;
          self.frames.last_mut().unwrap().stack.push(result);
        }
      }
      _ => {
        return Err(Error::TypeError {
          message: format!("'{}' object is not callable", function.type_name()),
        });
      }
    }

    Ok(())
  }

  fn compare_eq(&mut self) {
    let frame = self.frames.last_mut().unwrap();

    let rhs = frame.stack.pop().unwrap();
    let lhs = frame.stack.pop().unwrap();

    frame.stack.push(lhs.compare_eq(&rhs));
  }

  fn compare_ne(&mut self) {
    let frame = self.frames.last_mut().unwrap();

    let rhs = frame.stack.pop().unwrap();
    let lhs = frame.stack.pop().unwrap();

    frame.stack.push(lhs.compare_ne(&rhs));
  }

  fn dup(&mut self) {
    let frame = self.frames.last_mut().unwrap();

    let val = frame.stack.last().unwrap().clone();

    frame.stack.push(val);
  }

  fn execute(&mut self, code: Code) -> Result<Object> {
    let locals_len = code.locals.len();

    self.frames.push(Frame {
      code,
      ip: 0,
      locals: vec![None; locals_len],
      stack: Vec::new(),
    });

    self.run_loop()
  }

  fn execute_op(&mut self, op: Op) -> Result<Option<Object>> {
    match op {
      Op::BinaryAdd => self.binary_op(Object::binary_add)?,
      Op::BinaryDiv => self.binary_op(Object::binary_div)?,
      Op::BinaryFloorDiv => self.binary_op(Object::binary_floor_div)?,
      Op::BinaryMod => self.binary_op(Object::binary_mod)?,
      Op::BinaryMul => self.binary_op(Object::binary_mul)?,
      Op::BinaryPow => self.binary_op(Object::binary_pow)?,
      Op::BinarySub => self.binary_op(Object::binary_sub)?,
      Op::BuildString(count) => self.build_string(count),
      Op::CallFunction(argc) => self.call_function(argc)?,
      Op::CompareEq => self.compare_eq(),
      Op::CompareGe => self.binary_op(Object::compare_ge)?,
      Op::CompareGt => self.binary_op(Object::compare_gt)?,
      Op::CompareLe => self.binary_op(Object::compare_le)?,
      Op::CompareLt => self.binary_op(Object::compare_lt)?,
      Op::CompareNe => self.compare_ne(),
      Op::Dup => self.dup(),
      Op::Jump(target) => self.jump(target),
      Op::LoadConst(idx) | Op::MakeFunction(idx) => self.load_const(idx),
      Op::LoadFast(idx) => self.load_fast(idx)?,
      Op::LoadName(idx) => self.load_name(idx)?,
      Op::Pop => {
        self.frames.last_mut().unwrap().stack.pop();
      }
      Op::PopJumpIfFalse(target) => self.pop_jump_if_false(target),
      Op::PopJumpIfTrue(target) => self.pop_jump_if_true(target),
      Op::Return => return Ok(self.finish_frame()),
      Op::StoreFast(idx) => self.store_fast(idx),
      Op::StoreName(idx) => self.store_name(idx),
      Op::UnaryNeg => self.unary_neg()?,
      Op::UnaryNot => self.unary_not(),
      Op::UnaryPos => self.unary_pos()?,
    }

    Ok(None)
  }

  fn finish_frame(&mut self) -> Option<Object> {
    let frame = self.frames.pop().unwrap();

    let retval = frame.stack.into_iter().last().unwrap_or(Object::None);

    if self.frames.is_empty() {
      return Some(retval);
    }

    self.frames.last_mut().unwrap().stack.push(retval);

    None
  }

  fn initialize(&mut self) {
    self.globals.insert(
      "int".into(),
      Object::BuiltinFn(BuiltinFn {
        func: builtin_int,
        name: "int",
      }),
    );

    self.globals.insert(
      "len".into(),
      Object::BuiltinFn(BuiltinFn {
        func: builtin_len,
        name: "len",
      }),
    );

    self.globals.insert(
      "print".into(),
      Object::BuiltinFn(BuiltinFn {
        func: builtin_print,
        name: "print",
      }),
    );

    self.globals.insert(
      "str".into(),
      Object::BuiltinFn(BuiltinFn {
        func: builtin_str,
        name: "str",
      }),
    );

    self.globals.insert(
      "type".into(),
      Object::BuiltinFn(BuiltinFn {
        func: builtin_type,
        name: "type",
      }),
    );
  }

  fn jump(&mut self, target: u16) {
    self.frames.last_mut().unwrap().ip = usize::from(target);
  }

  fn load_const(&mut self, idx: u16) {
    let val =
      self.frames.last().unwrap().code.constants[usize::from(idx)].clone();

    self.frames.last_mut().unwrap().stack.push(val);
  }

  fn load_fast(&mut self, idx: u16) -> Result<()> {
    let frame = self.frames.last_mut().unwrap();

    let idx = usize::from(idx);

    let val = frame.locals[idx]
      .clone()
      .ok_or_else(|| Error::UnboundLocal {
        name: frame.code.locals[idx].clone(),
      })?;

    frame.stack.push(val);

    Ok(())
  }

  fn load_name(&mut self, idx: u16) -> Result<()> {
    let name = self.frames.last().unwrap().code.names[usize::from(idx)].clone();

    self.frames.last_mut().unwrap().stack.push(
      self
        .globals
        .get(&name)
        .ok_or(Error::NameError { name })?
        .clone(),
    );

    Ok(())
  }

  fn next_op(&mut self) -> Op {
    let frame = self.frames.last().unwrap();

    let op = frame.code.ops[frame.ip];

    self.frames.last_mut().unwrap().ip += 1;

    op
  }

  fn pop_jump_if_false(&mut self, target: u16) {
    let frame = self.frames.last_mut().unwrap();

    if !frame.stack.pop().unwrap().is_truthy() {
      frame.ip = usize::from(target);
    }
  }

  fn pop_jump_if_true(&mut self, target: u16) {
    let frame = self.frames.last_mut().unwrap();

    if frame.stack.pop().unwrap().is_truthy() {
      frame.ip = usize::from(target);
    }
  }

  fn run_loop(&mut self) -> Result<Object> {
    loop {
      if let Some(result) = self.step()? {
        return Ok(result);
      }
    }
  }

  fn step(&mut self) -> Result<Option<Object>> {
    let frame = self.frames.last().unwrap();

    if frame.ip >= frame.code.ops.len() {
      return Ok(self.finish_frame());
    }

    let op = self.next_op();

    self.execute_op(op)
  }

  fn store_fast(&mut self, idx: u16) {
    let frame = self.frames.last_mut().unwrap();

    let val = frame.stack.pop().unwrap();

    frame.locals[usize::from(idx)] = Some(val);
  }

  fn store_name(&mut self, idx: u16) {
    let frame = self.frames.last_mut().unwrap();

    let name = frame.code.names[usize::from(idx)].clone();

    let val = frame.stack.pop().unwrap();

    self.globals.insert(name, val);
  }

  fn unary_neg(&mut self) -> Result<()> {
    let frame = self.frames.last_mut().unwrap();

    let val = frame.stack.pop().unwrap();

    frame.stack.push(val.unary_neg()?);

    Ok(())
  }

  fn unary_not(&mut self) {
    let frame = self.frames.last_mut().unwrap();

    let val = frame.stack.pop().unwrap();

    frame.stack.push(val.unary_not());
  }

  fn unary_pos(&mut self) -> Result<()> {
    let frame = self.frames.last_mut().unwrap();

    let val = frame.stack.pop().unwrap();

    frame.stack.push(val.unary_pos()?);

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

fn builtin_int(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError {
      message: "int() takes exactly one argument".into(),
    });
  }

  match &args[0] {
    Object::Int(integer) => Ok(Object::Int(*integer)),
    Object::Float(float) => {
      float.to_i64().map(Object::Int).ok_or(Error::Overflow)
    }
    Object::Bool(boolean) => Ok(Object::Int(i64::from(*boolean))),
    Object::Str(string) => {
      string
        .parse::<i64>()
        .map(Object::Int)
        .map_err(|_| Error::TypeError {
          message: format!("invalid literal for int(): '{string}'"),
        })
    }
    _ => Err(Error::TypeError {
      message: format!(
        "int() argument must be a string or a number, not '{}'",
        args[0].type_name()
      ),
    }),
  }
}

fn builtin_len(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError {
      message: "len() takes exactly one argument".into(),
    });
  }

  match &args[0] {
    Object::Str(s) => i64::try_from(s.len())
      .map(Object::Int)
      .map_err(|_| Error::Overflow),
    _ => Err(Error::TypeError {
      message: format!("object of type '{}' has no len()", args[0].type_name()),
    }),
  }
}

fn builtin_print(args: &[Object]) -> Result<Object> {
  write_print(args, &mut io::stdout())?;
  Ok(Object::None)
}

fn builtin_str(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError {
      message: "str() takes exactly one argument".into(),
    });
  }

  Ok(Object::Str(args[0].to_string()))
}

fn builtin_type(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError {
      message: "type() takes exactly one argument".into(),
    });
  }

  Ok(Object::Str(format!("<class '{}'>", args[0].type_name())))
}

fn write_print<W: Write>(arguments: &[Object], output: &mut W) -> Result<()> {
  writeln!(
    output,
    "{}",
    arguments
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>()
      .join(" ")
  )
  .map_err(|source| Error::Io { source })
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    ruff_python_parser::{Mode, parse},
  };

  fn run(source: &str) -> (Object, String) {
    let code = Compiler::compile(
      parse(source, Mode::Module.into())
        .unwrap()
        .try_into_module()
        .unwrap()
        .syntax(),
    )
    .unwrap();

    let output = Vec::new();

    let (result, output) = Machine::with_output(code, output).unwrap();

    (result, String::from_utf8(output).unwrap())
  }

  #[test]
  fn arithmetic() {
    let (_, output) = run("print(1 + 2)\n");

    assert_eq!(output, "3\n");
  }

  #[test]
  fn aug_assign() {
    let (_, output) = run("foo = 10\nfoo += 5\nprint(foo)\n");

    assert_eq!(output, "15\n");
  }

  #[test]
  fn bool_ops() {
    #[track_caller]
    fn case(source: &str, expected: &str) {
      let (_, output) = run(source);
      assert_eq!(output, expected);
    }

    case("print(1 and 2)\n", "2\n");
    case("print(0 and 2)\n", "0\n");
    case("print(1 or 2)\n", "1\n");
    case("print(0 or 2)\n", "2\n");
  }

  #[test]
  fn comparison_ops() {
    #[track_caller]
    fn case(source: &str, expected: &str) {
      let (_, output) = run(source);
      assert_eq!(output, expected);
    }

    case("print(1 < 2)\n", "True\n");
    case("print(2 < 1)\n", "False\n");
    case("print(1 == 1)\n", "True\n");
    case("print(1 != 2)\n", "True\n");
  }

  #[test]
  fn function_call() {
    let (_, output) =
      run("def foo(bar):\n    return bar + 1\nprint(foo(41))\n");

    assert_eq!(output, "42\n");
  }

  #[test]
  fn greet_example() {
    let source = r#"
x = 1 + 2
print(x)

def greet(name):
    return "Hello, " + name

print(greet("world"))
"#;
    let (_, output) = run(source);

    assert_eq!(output, "3\nHello, world\n");
  }

  #[test]
  fn if_else() {
    let (_, output) =
      run("if 0:\n    print(\"foo\")\nelse:\n    print(\"bar\")\n");

    assert_eq!(output, "bar\n");
  }

  #[test]
  fn if_false_branch() {
    let (_, output) = run("if 0:\n    print(\"foo\")\n");

    assert_eq!(output, "");
  }

  #[test]
  fn if_true_branch() {
    let (_, output) = run("if 1:\n    print(\"foo\")\n");

    assert_eq!(output, "foo\n");
  }

  #[test]
  fn implicit_return() {
    let (_, output) = run("def foo():\n    pass\nprint(foo())\n");

    assert_eq!(output, "None\n");
  }

  #[test]
  fn multiple_args() {
    let (_, output) = run("print(\"foo\", \"bar\", \"baz\")\n");

    assert_eq!(output, "foo bar baz\n");
  }

  #[test]
  fn name_error() {
    let parsed = parse("foo\n", Mode::Module.into())
      .unwrap()
      .try_into_module()
      .unwrap();

    let result = Machine::run(Compiler::compile(parsed.syntax()).unwrap());

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("foo"));
  }

  #[test]
  fn nested_function() {
    let (_, output) = run(
      "def foo(bar):\n    def baz(qux):\n        return qux * 2\n    return baz(bar) + 1\nprint(foo(5))\n",
    );

    assert_eq!(output, "11\n");
  }

  #[test]
  fn string_concatenation() {
    let (_, output) = run("print(\"foo\" + \"bar\")\n");

    assert_eq!(output, "foobar\n");
  }

  #[test]
  fn ternary() {
    #[track_caller]
    fn case(source: &str, expected: &str) {
      let (_, output) = run(source);
      assert_eq!(output, expected);
    }

    case("print(\"foo\" if 1 else \"bar\")\n", "foo\n");
    case("print(\"foo\" if 0 else \"bar\")\n", "bar\n");
  }

  #[test]
  fn variable_assignment() {
    let (_, output) = run("foo = 42\nprint(foo)\n");

    assert_eq!(output, "42\n");
  }

  #[test]
  fn while_loop() {
    let (_, output) =
      run("foo = 0\nwhile foo < 3:\n    print(foo)\n    foo += 1\n");

    assert_eq!(output, "0\n1\n2\n");
  }
}
