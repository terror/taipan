use {
  ruff_python_ast::ModModule,
  ruff_python_parser::{Mode, ParseError, Parsed, parse},
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
def hello(name):
    print(f"Hello, {name}!")

hello("world")
"#;

  match parse_module(source) {
    Ok(parsed) => {
      println!("{:#?}", parsed.syntax());
    }
    Err(error) => {
      eprintln!("error: {error}");
    }
  }
}
