use {
  ruff_python_ast::ModModule,
  ruff_python_parser::{Mode, ParseError, Parsed, parse},
  std::process,
  taipan::{Compiler, Vm},
};

fn parse_module(source: &str) -> Result<Parsed<ModModule>, ParseError> {
  Ok(
    parse(source, Mode::Module.into())?
      .try_into_module()
      .expect("Mode::Module should produce ModModule"),
  )
}

fn main() {
  let source = r#"
x = 1 + 2
print(x)

def greet(name):
    return "Hello, " + name

print(greet("world"))
"#;

  let parsed = match parse_module(source) {
    Ok(parsed) => parsed,
    Err(error) => {
      eprintln!("SyntaxError: {error}");
      process::exit(1);
    }
  };

  let code = match Compiler::compile(parsed.syntax()) {
    Ok(code) => code,
    Err(error) => {
      eprintln!("{error}");
      process::exit(1);
    }
  };

  if let Err(error) = Vm::run(code) {
    eprintln!("{error}");
    process::exit(1);
  }
}
