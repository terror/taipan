use {
  Match::*,
  executable_path::executable_path,
  pretty_assertions::assert_eq,
  std::{fs::File, io::Write, process::Command, str},
  tempfile::TempDir,
  unindent::Unindent,
};

#[derive(Clone, Debug)]
enum Match<'a> {
  Contains(&'a str),
  Empty,
  Exact(&'a str),
}

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

struct Test<'a> {
  arguments: Vec<String>,
  expected_status: i32,
  expected_stderr: Match<'a>,
  expected_stdout: Match<'a>,
  program: &'a str,
  tempdir: TempDir,
}

impl<'a> Test<'a> {
  fn argument(mut self, argument: &str) -> Self {
    self.arguments.push(argument.to_owned());
    self
  }

  fn expected_status(self, expected_status: i32) -> Self {
    Self {
      expected_status,
      ..self
    }
  }

  fn expected_stderr(self, expected_stderr: Match<'a>) -> Self {
    Self {
      expected_stderr,
      ..self
    }
  }

  fn expected_stdout(self, expected_stdout: Match<'a>) -> Self {
    Self {
      expected_stdout,
      ..self
    }
  }

  fn new() -> Result<Self> {
    Ok(Self {
      arguments: Vec::new(),
      expected_status: 0,
      expected_stderr: Match::Empty,
      expected_stdout: Match::Empty,
      program: "",
      tempdir: TempDir::new()?,
    })
  }

  fn program(self, program: &'a str) -> Self {
    Self { program, ..self }
  }

  fn run(self) -> Result {
    let mut command = Command::new(executable_path(env!("CARGO_PKG_NAME")));

    for argument in self.arguments {
      command.arg(argument);
    }

    if !self.program.is_empty() {
      let program_path = self.tempdir.path().join("program.py");

      let mut file = File::create(&program_path)?;

      write!(file, "{}", self.program.unindent())?;

      command.arg(&program_path);
    }

    let output = command.output().map_err(|error| {
      format!(
        "failed to execute command `{}`: {error}",
        command.get_program().to_string_lossy(),
      )
    })?;

    let stderr = str::from_utf8(&output.stderr)?;

    match &self.expected_stderr {
      Match::Empty => {
        assert!(
          stderr.is_empty(),
          "expected empty stderr, but received: {stderr}"
        );
      }
      Match::Contains(pattern) => {
        assert!(
          stderr.contains(pattern),
          "expected stderr to contain: `{pattern}`, but got: `{stderr}`",
        );
      }
      Match::Exact(expected) => {
        assert_eq!(
          stderr, *expected,
          "expected exact stderr: `{expected}`, but got: `{stderr}`",
        );
      }
    }

    let stdout = str::from_utf8(&output.stdout)?;

    match &self.expected_stdout {
      Match::Empty => {
        assert!(
          stdout.is_empty(),
          "expected empty stdout, but received: {stdout}"
        );
      }
      Match::Contains(pattern) => {
        assert!(
          stdout.contains(pattern),
          "expected stdout to contain: `{pattern}`, but got: `{stdout}`",
        );
      }
      Match::Exact(expected) => {
        assert_eq!(
          stdout, *expected,
          "expected exact stdout: `{expected}`, but got: `{stdout}`",
        );
      }
    }

    assert_eq!(output.status.code(), Some(self.expected_status));

    Ok(())
  }
}

#[test]
fn abs_builtin() -> Result {
  Test::new()?
    .program(
      "
      print(abs(-1))
      print(abs(-1.5))
      ",
    )
    .expected_stdout(Exact("1\n1.5\n"))
    .run()
}

#[test]
fn arithmetic() -> Result {
  Test::new()?
    .program(
      "
      print(1 + 2)
      print(5 - 3)
      print(3 * 4)
      print(7 / 2)
      print(7 // 2)
      print(7 % 3)
      print(2 ** 10)
      print(-1)
      print(+1)
      print(1.5 + 2.5)
      print(1 + 2.5)
      ",
    )
    .expected_stdout(Exact("3\n2\n12\n3.5\n3\n1\n1024\n-1\n1\n4.0\n3.5\n"))
    .run()
}

#[test]
fn bitwise() -> Result {
  Test::new()?
    .program(
      "
      print(6 & 3)
      print(4 | 1)
      print(7 ^ 3)
      print(3 << 2)
      print(-8 >> 2)
      print(~2)
      ",
    )
    .expected_stdout(Exact("2\n5\n4\n12\n-2\n-3\n"))
    .run()
}

#[test]
fn bool_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(bool())
      print(bool(0))
      print(bool(1))
      print(bool(""))
      print(bool("foo"))
      "#,
    )
    .expected_stdout(Exact("False\nFalse\nTrue\nFalse\nTrue\n"))
    .run()
}

#[test]
fn bool_ops() -> Result {
  Test::new()?
    .program(
      "
      print(1 and 2)
      print(0 and 2)
      print(1 or 2)
      print(0 or 2)
      print(not 0)
      print(not 1)
      ",
    )
    .expected_stdout(Exact("2\n0\n1\n2\nTrue\nFalse\n"))
    .run()
}

#[test]
fn break_continue() -> Result {
  Test::new()?
    .program(
      "
      foo = 0
      while foo < 5:
        foo += 1
        if foo == 2:
          continue
        if foo == 4:
          break
        print(foo)
      else:
        print('bar')
      ",
    )
    .expected_stdout(Exact("1\n3\n"))
    .run()
}

#[test]
fn break_in_nested_function() -> Result {
  Test::new()?
    .program(
      "
      while foo:
        def bar():
          break
      ",
    )
    .expected_status(1)
    .expected_stderr(Contains("CompileError: break outside loop"))
    .run()
}

#[test]
fn break_outside_loop() -> Result {
  Test::new()?
    .program("break")
    .expected_status(1)
    .expected_stderr(Contains("CompileError: break outside loop"))
    .run()
}

#[test]
fn comparisons() -> Result {
  Test::new()?
    .program(
      r#"
      print(1 < 2)
      print(2 < 1)
      print(1 == 1)
      print(1 == 1.0)
      print(1 != 2)
      print("foo" < "bar")
      print("foo" > "bar")
      "#,
    )
    .expected_stdout(Exact("True\nFalse\nTrue\nTrue\nTrue\nFalse\nTrue\n"))
    .run()
}

#[test]
fn continue_in_nested_function() -> Result {
  Test::new()?
    .program(
      "
      while foo:
        def bar():
          continue
      ",
    )
    .expected_status(1)
    .expected_stderr(Contains("CompileError: continue outside loop"))
    .run()
}

#[test]
fn continue_outside_loop() -> Result {
  Test::new()?
    .program("continue")
    .expected_status(1)
    .expected_stderr(Contains("CompileError: continue outside loop"))
    .run()
}

#[test]
fn control_flow() -> Result {
  Test::new()?
    .program(
      r#"
      if 0:
        print("foo")
      elif 1:
        print("bar")
      else:
        print("baz")

      if 1:
        print("qux")

      print("foo" if 1 else "bar")
      print("foo" if 0 else "bar")
      "#,
    )
    .expected_stdout(Exact("bar\nqux\nfoo\nbar\n"))
    .run()
}

#[test]
fn closure_capture() -> Result {
  Test::new()?
    .program(
      "
      def foo():
        bar = 1

        def baz():
          return bar

        return baz()

      print(foo())
      ",
    )
    .expected_stdout(Exact("1\n"))
    .run()
}

#[test]
fn display_values() -> Result {
  Test::new()?
    .program(
      r#"
      print(None)
      print(True)
      print(False)
      print(42)
      print(3.0)
      print(1.5)
      print("foo")
      "#,
    )
    .expected_stdout(Exact("None\nTrue\nFalse\n42\n3.0\n1.5\nfoo\n"))
    .run()
}

#[test]
fn float_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(float())
      print(float(1))
      print(float("1.5"))
      print(float(True))
      "#,
    )
    .expected_stdout(Exact("0.0\n1.0\n1.5\n1.0\n"))
    .run()
}

#[test]
fn int_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(int())
      print(int(1.5))
      print(int("2"))
      print(int(True))
      "#,
    )
    .expected_stdout(Exact("0\n1\n2\n1\n"))
    .run()
}

#[test]
fn f_strings() -> Result {
  Test::new()?
    .program(
      r#"
      foo = "foo"
      bar = 42
      print(f"{foo} {bar + 1}")
      print(f"foo {bar} baz")
      print(f"foo={foo!r}")
      print(f"{bar=}")
      print(f"{bar + 1=}")
      print(f"{ foo = }")
      "#,
    )
    .expected_stdout(Exact(
      "foo 43\nfoo 42 baz\nfoo=foo\nbar=42\nbar + 1=43\n foo = foo\n",
    ))
    .run()
}

#[test]
fn function_call() -> Result {
  Test::new()?
    .program(
      "
      def foo(bar):
        return bar + 1

      print(foo(41))
      ",
    )
    .expected_stdout(Exact("42\n"))
    .run()
}

#[test]
fn function_default_arguments() -> Result {
  Test::new()?
    .program(
      "
      foo = 1

      def bar(baz, qux=foo + 1):
        return baz + qux

      foo = 10

      print(bar(1))
      print(bar(1, 3))
      ",
    )
    .expected_stdout(Exact("3\n4\n"))
    .run()
}

#[test]
fn for_loop() -> Result {
  Test::new()?
    .program(
      "
      for foo in [1, 2, 3]:
        print(foo)

      for foo in 'bar':
        print(foo)

      for foo in range(2):
        print(foo)

      for foo in range(2, 5):
        print(foo)

      for foo in range(5, 2, -2):
        print(foo)
      ",
    )
    .expected_stdout(Exact("1\n2\n3\nb\na\nr\n0\n1\n2\n3\n4\n5\n3\n"))
    .run()
}

#[test]
fn for_loop_break_continue_else() -> Result {
  Test::new()?
    .program(
      "
      for foo in [1, 2, 3]:
        if foo == 1:
          continue
        if foo == 3:
          break
        print(foo)
      else:
        print('foo')

      for foo in [1]:
        print(foo)
      else:
        print('bar')
      ",
    )
    .expected_stdout(Exact("2\n1\nbar\n"))
    .run()
}

#[test]
fn lists() -> Result {
  Test::new()?
    .program(
      r#"
      foo = [1, "bar", 3]
      print(foo)
      print(foo[0])
      print(foo[-1])
      print(len(foo))
      print(bool([]))
      print(bool([1]))
      print([1, 2] == [1, 2])
      print([1, 2] == [2, 1])
      "#,
    )
    .expected_stdout(Exact("[1, bar, 3]\n1\n3\n3\nFalse\nTrue\nTrue\nFalse\n"))
    .run()
}

#[test]
fn list_assignment() -> Result {
  Test::new()?
    .program(
      "
      foo = [1, 2, 3]
      bar = foo
      bar[1] = 4
      foo[2] += 5
      print(foo)
      print(bar)
      ",
    )
    .expected_stdout(Exact("[1, 4, 8]\n[1, 4, 8]\n"))
    .run()
}

#[test]
fn list_operations() -> Result {
  Test::new()?
    .program(
      "
      print([1] + [2, 3])
      print([1, 2] * 2)
      print(2 * [3])
      ",
    )
    .expected_stdout(Exact("[1, 2, 3]\n[1, 2, 1, 2]\n[3, 3]\n"))
    .run()
}

#[test]
fn tuple_assignment() -> Result {
  Test::new()?
    .program(
      "
      foo, bar = 1, 2
      foo, bar = bar, foo
      [baz, [qux, quux]] = [3, [4, 5]]
      print(foo)
      print(bar)
      print(baz)
      print(qux)
      print(quux)
      ",
    )
    .expected_stdout(Exact("2\n1\n3\n4\n5\n"))
    .run()
}

#[test]
fn tuple_assignment_too_few_values() -> Result {
  Test::new()?
    .program(
      "
      foo, bar = [1]
      ",
    )
    .expected_status(1)
    .expected_stderr(Contains(
      "TypeError: not enough values to unpack (expected 2, got 1)",
    ))
    .run()
}

#[test]
fn tuple_assignment_too_many_values() -> Result {
  Test::new()?
    .program(
      "
      foo, bar = [1, 2, 3]
      ",
    )
    .expected_status(1)
    .expected_stderr(Contains(
      "TypeError: too many values to unpack (expected 2, got 3)",
    ))
    .run()
}

#[test]
fn tuple_for_loop() -> Result {
  Test::new()?
    .program(
      "
      for foo, bar in [(1, 2), (3, 4)]:
        print(foo)
        print(bar)
      ",
    )
    .expected_stdout(Exact("1\n2\n3\n4\n"))
    .run()
}

#[test]
fn tuple_operations() -> Result {
  Test::new()?
    .program(
      "
      print((1, 2) + (3,))
      print((1, 2) * 2)
      print(2 * (3,))
      ",
    )
    .expected_stdout(Exact("(1, 2, 3)\n(1, 2, 1, 2)\n(3, 3)\n"))
    .run()
}

#[test]
fn tuples() -> Result {
  Test::new()?
    .program(
      "
      foo = (1, 'bar', 3)
      print(())
      print((1,))
      print(foo)
      print(foo[0])
      print(foo[-1])
      print(len(foo))
      print(bool(()))
      print(bool((1,)))
      print((1, 2) == (1, 2))
      print((1, 2) == (2, 1))
      ",
    )
    .expected_stdout(Exact(
      "()\n(1,)\n(1, bar, 3)\n1\n3\n3\nFalse\nTrue\nTrue\nFalse\n",
    ))
    .run()
}

#[test]
fn implicit_return() -> Result {
  Test::new()?
    .program(
      "
      def foo():
        pass

      print(foo())
      ",
    )
    .expected_stdout(Exact("None\n"))
    .run()
}

#[test]
fn max_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(max(1, 3, 2))
      print(max("foo", "bar"))
      "#,
    )
    .expected_stdout(Exact("3\nfoo\n"))
    .run()
}

#[test]
fn min_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(min(3, 1, 2))
      print(min("foo", "bar"))
      "#,
    )
    .expected_stdout(Exact("1\nbar\n"))
    .run()
}

#[test]
fn missing_program_path() -> Result {
  Test::new()?
    .expected_status(2)
    .expected_stderr(Contains(" <FILENAME>"))
    .run()
}

#[test]
fn multiple_args() -> Result {
  Test::new()?
    .program(r#"print("foo", "bar", "baz")"#)
    .expected_stdout(Exact("foo bar baz\n"))
    .run()
}

#[test]
fn name_error() -> Result {
  Test::new()?
    .program("print(foo)")
    .expected_status(1)
    .expected_stderr(Contains("NameError: name 'foo' is not defined"))
    .run()
}

#[test]
fn nested_function() -> Result {
  Test::new()?
    .program(
      "
      def foo(bar):
        def baz(qux):
          return qux * 2
        return baz(bar) + 1

      print(foo(5))
      ",
    )
    .expected_stdout(Exact("11\n"))
    .run()
}

#[test]
fn nested_closure_capture() -> Result {
  Test::new()?
    .program(
      "
      def foo():
        bar = 1

        def baz():
          def qux():
            return bar

          return qux()

        return baz()

      print(foo())
      ",
    )
    .expected_stdout(Exact("1\n"))
    .run()
}

#[test]
fn nonlocal_assignment() -> Result {
  Test::new()?
    .program(
      "
      def foo():
        bar = 1

        def baz():
          nonlocal bar
          bar += 1

        baz()
        print(bar)

      foo()
      ",
    )
    .expected_stdout(Exact("2\n"))
    .run()
}

#[test]
fn nonlocal_missing_binding() -> Result {
  Test::new()?
    .program(
      "
      def foo():
        def bar():
          nonlocal baz
      ",
    )
    .expected_status(1)
    .expected_stderr(Contains(
      "CompileError: no binding for nonlocal 'baz' found",
    ))
    .run()
}

#[test]
fn program_file() -> Result {
  Test::new()?
    .program(
      "
      print(1 + 2)
      ",
    )
    .expected_stdout(Exact("3\n"))
    .run()
}

#[test]
fn repr_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(repr("foo"))
      print(repr(1.5))
      "#,
    )
    .expected_stdout(Exact("foo\n1.5\n"))
    .run()
}

#[test]
fn str_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print("foo" + str() + "bar")
      print(str(1))
      "#,
    )
    .expected_stdout(Exact("foobar\n1\n"))
    .run()
}

#[test]
fn string_concatenation() -> Result {
  Test::new()?
    .program(r#"print("foo" + "bar")"#)
    .expected_stdout(Exact("foobar\n"))
    .run()
}

#[test]
fn string_repetition() -> Result {
  Test::new()?
    .program(
      r#"
      print("foo" * 3)
      print(3 * "bar")
      "#,
    )
    .expected_stdout(Exact("foofoofoo\nbarbarbar\n"))
    .run()
}

#[test]
fn too_many_program_paths() -> Result {
  Test::new()?
    .argument("foo.py")
    .program("print(1)")
    .expected_status(2)
    .expected_stderr(Contains("unexpected argument"))
    .run()
}

#[test]
fn top_level_return() -> Result {
  Test::new()?
    .program("return")
    .expected_status(1)
    .expected_stderr(Contains("CompileError: 'return' outside function"))
    .run()
}

#[test]
fn type_errors() -> Result {
  #[track_caller]
  fn case(program: &str, expected: &str) -> Result {
    Test::new()?
      .program(program)
      .expected_status(1)
      .expected_stderr(Contains(expected))
      .run()
  }

  case(r#"1 + "foo""#, "unsupported operand type(s) for +")?;
  case("1 / 0", "division by zero")?;
  case("1 // 0", "integer division or modulo by zero")?;
  case("1 % 0", "integer division or modulo by zero")?;
  case(r#""foo" < 1"#, "'<' not supported between instances")
}

#[test]
fn variable_assignment() -> Result {
  Test::new()?
    .program(
      "
      foo = 42
      print(foo)
      ",
    )
    .expected_stdout(Exact("42\n"))
    .run()
}

#[test]
fn while_else() -> Result {
  Test::new()?
    .program(
      "
      while 0:
        print('foo')
      else:
        print('bar')
      ",
    )
    .expected_stdout(Exact("bar\n"))
    .run()
}

#[test]
fn while_loop() -> Result {
  Test::new()?
    .program(
      "
      foo = 0
      while foo < 3:
        print(foo)
        foo += 1
      ",
    )
    .expected_stdout(Exact("0\n1\n2\n"))
    .run()
}
