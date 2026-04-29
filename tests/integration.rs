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
fn missing_program_path() -> Result {
  Test::new()?
    .expected_status(2)
    .expected_stderr(Contains(" <FILENAME>"))
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
fn bool_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(bool(0))
      print(bool(1))
      print(bool(""))
      print(bool("foo"))
      "#,
    )
    .expected_stdout(Exact("False\nTrue\nFalse\nTrue\n"))
    .run()
}

#[test]
fn float_builtin() -> Result {
  Test::new()?
    .program(
      r#"
      print(float(1))
      print(float("1.5"))
      print(float(True))
      "#,
    )
    .expected_stdout(Exact("1.0\n1.5\n1.0\n"))
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
fn break_outside_loop() -> Result {
  Test::new()?
    .program("break")
    .expected_status(1)
    .expected_stderr(Contains("CompileError: break outside loop"))
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
