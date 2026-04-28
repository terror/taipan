use super::*;

struct Frame {
  code: Code,
  ip: usize,
  locals: Vec<Option<Object>>,
  stack: Vec<Object>,
}

pub struct Vm<W: Write> {
  frames: Vec<Frame>,
  globals: HashMap<String, Object>,
  output: W,
}

impl Vm<io::Stdout> {
  pub fn run(code: Code) -> Result<Object> {
    let mut vm = Vm {
      frames: Vec::new(),
      globals: HashMap::new(),
      output: io::stdout(),
    };

    vm.init_builtins();

    vm.execute(code)
  }
}

impl<W: Write> Vm<W> {
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

  fn call_function(&mut self, argc: u8) -> Result<()> {
    let argc = argc as usize;
    let frame = self.frames.last_mut().unwrap();
    let args = frame.stack.split_off(frame.stack.len() - argc);
    let func = frame.stack.pop().unwrap();

    match func {
      Object::Function {
        name: _,
        params,
        code,
      } => {
        if params.len() != argc {
          return Err(Error::TypeError(format!(
            "expected {} arguments, got {argc}",
            params.len()
          )));
        }
        let mut locals = vec![None; code.locals.len()];
        for (i, arg) in args.into_iter().enumerate() {
          locals[i] = Some(arg);
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
          let parts = args.iter().map(ToString::to_string).collect::<Vec<_>>();
          writeln!(self.output, "{}", parts.join(" ")).ok();
          self.frames.last_mut().unwrap().stack.push(Object::None);
        } else {
          let result = (bf.func)(&args)?;
          self.frames.last_mut().unwrap().stack.push(result);
        }
      }
      _ => {
        return Err(Error::TypeError(format!(
          "'{}' object is not callable",
          func.type_name()
        )));
      }
    }

    Ok(())
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

  fn init_builtins(&mut self) {
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

  fn run_loop(&mut self) -> Result<Object> {
    loop {
      let frame = self.frames.last().unwrap();
      if frame.ip >= frame.code.ops.len() {
        let frame = self.frames.pop().unwrap();
        let retval = frame.stack.into_iter().last().unwrap_or(Object::None);
        if self.frames.is_empty() {
          return Ok(retval);
        }
        self.frames.last_mut().unwrap().stack.push(retval);
        continue;
      }

      let op = frame.code.ops[frame.ip];
      self.frames.last_mut().unwrap().ip += 1;

      match op {
        Op::BinaryAdd => self.binary_op(Object::binary_add)?,
        Op::BinaryDiv => self.binary_op(Object::binary_div)?,
        Op::BinaryFloorDiv => self.binary_op(Object::binary_floor_div)?,
        Op::BinaryMod => self.binary_op(Object::binary_mod)?,
        Op::BinaryMul => self.binary_op(Object::binary_mul)?,
        Op::BinaryPow => self.binary_op(Object::binary_pow)?,
        Op::BinarySub => self.binary_op(Object::binary_sub)?,
        Op::BuildString(count) => {
          let frame = self.frames.last_mut().unwrap();
          let start = frame.stack.len() - count as usize;
          let parts: String = frame.stack[start..]
            .iter()
            .map(ToString::to_string)
            .collect();
          frame.stack.truncate(start);
          frame.stack.push(Object::Str(parts));
        }
        Op::CallFunction(argc) => self.call_function(argc)?,
        Op::CompareEq => {
          let frame = self.frames.last_mut().unwrap();
          let rhs = frame.stack.pop().unwrap();
          let lhs = frame.stack.pop().unwrap();
          frame.stack.push(lhs.compare_eq(&rhs));
        }
        Op::CompareGe => self.binary_op(Object::compare_ge)?,
        Op::CompareGt => self.binary_op(Object::compare_gt)?,
        Op::CompareLe => self.binary_op(Object::compare_le)?,
        Op::CompareLt => self.binary_op(Object::compare_lt)?,
        Op::CompareNe => {
          let frame = self.frames.last_mut().unwrap();
          let rhs = frame.stack.pop().unwrap();
          let lhs = frame.stack.pop().unwrap();
          frame.stack.push(lhs.compare_ne(&rhs));
        }
        Op::Dup => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.last().unwrap().clone();
          frame.stack.push(val);
        }
        Op::Jump(target) => {
          self.frames.last_mut().unwrap().ip = target as usize;
        }
        Op::LoadConst(idx) | Op::MakeFunction(idx) => {
          let val =
            self.frames.last().unwrap().code.constants[idx as usize].clone();
          self.frames.last_mut().unwrap().stack.push(val);
        }
        Op::LoadFast(idx) => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.locals[idx as usize].clone().ok_or_else(|| {
            Error::UnboundLocal(frame.code.locals[idx as usize].clone())
          })?;
          frame.stack.push(val);
        }
        Op::LoadName(idx) => {
          let name =
            self.frames.last().unwrap().code.names[idx as usize].clone();
          let val = self
            .globals
            .get(&name)
            .ok_or(Error::NameError(name))?
            .clone();
          self.frames.last_mut().unwrap().stack.push(val);
        }
        Op::Pop => {
          self.frames.last_mut().unwrap().stack.pop();
        }
        Op::PopJumpIfFalse(target) => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.pop().unwrap();
          if !val.is_truthy() {
            frame.ip = target as usize;
          }
        }
        Op::PopJumpIfTrue(target) => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.pop().unwrap();
          if val.is_truthy() {
            frame.ip = target as usize;
          }
        }
        Op::Return => {
          let frame = self.frames.pop().unwrap();
          let retval = frame.stack.into_iter().last().unwrap_or(Object::None);
          if self.frames.is_empty() {
            return Ok(retval);
          }
          self.frames.last_mut().unwrap().stack.push(retval);
        }
        Op::StoreFast(idx) => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.pop().unwrap();
          frame.locals[idx as usize] = Some(val);
        }
        Op::StoreName(idx) => {
          let frame = self.frames.last_mut().unwrap();
          let name = frame.code.names[idx as usize].clone();
          let val = frame.stack.pop().unwrap();
          self.globals.insert(name, val);
        }
        Op::UnaryNeg => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.pop().unwrap();
          frame.stack.push(val.unary_neg()?);
        }
        Op::UnaryNot => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.pop().unwrap();
          frame.stack.push(val.unary_not());
        }
        Op::UnaryPos => {
          let frame = self.frames.last_mut().unwrap();
          let val = frame.stack.pop().unwrap();
          frame.stack.push(val.unary_pos()?);
        }
      }
    }
  }

  pub fn with_output(code: Code, output: W) -> Result<(Object, W)> {
    let mut vm = Vm {
      frames: Vec::new(),
      globals: HashMap::new(),
      output,
    };
    vm.init_builtins();
    let result = vm.execute(code)?;
    Ok((result, vm.output))
  }
}

fn builtin_int(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError("int() takes exactly one argument".into()));
  }

  match &args[0] {
    Object::Int(i) => Ok(Object::Int(*i)),
    Object::Float(f) => Ok(Object::Int(*f as i64)),
    Object::Bool(b) => Ok(Object::Int(i64::from(*b))),
    Object::Str(s) => s.parse::<i64>().map(Object::Int).map_err(|_| {
      Error::TypeError(format!("invalid literal for int(): '{s}'"))
    }),
    _ => Err(Error::TypeError(format!(
      "int() argument must be a string or a number, not '{}'",
      args[0].type_name()
    ))),
  }
}

fn builtin_len(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError("len() takes exactly one argument".into()));
  }

  match &args[0] {
    Object::Str(s) => Ok(Object::Int(s.len() as i64)),
    _ => Err(Error::TypeError(format!(
      "object of type '{}' has no len()",
      args[0].type_name()
    ))),
  }
}

fn builtin_print(args: &[Object]) -> Result<Object> {
  let parts = args.iter().map(ToString::to_string).collect::<Vec<_>>();
  println!("{}", parts.join(" "));
  Ok(Object::None)
}

fn builtin_str(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError("str() takes exactly one argument".into()));
  }

  Ok(Object::Str(args[0].to_string()))
}

fn builtin_type(args: &[Object]) -> Result<Object> {
  if args.len() != 1 {
    return Err(Error::TypeError("type() takes exactly one argument".into()));
  }

  Ok(Object::Str(format!("<class '{}'>", args[0].type_name())))
}

#[cfg(test)]
mod tests {
  use ruff_python_parser::{Mode, parse};

  use crate::Compiler;

  use super::*;

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

    let (result, output) = Vm::with_output(code, output).unwrap();

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
    let code = Compiler::compile(parsed.syntax()).unwrap();
    let result = Vm::run(code);
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
