use super::*;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
  #[snafu(display("CompileError: {message}"))]
  Compile { message: String },
  #[snafu(display("IOError: {source}"))]
  Io { source: io::Error },
  #[snafu(display("NameError: name '{name}' is not defined"))]
  NameError { name: String },
  #[snafu(display("OverflowError: integer overflow"))]
  Overflow,
  #[snafu(display("SyntaxError: {source}"))]
  Parse { source: ParseError },
  #[snafu(display("TypeError: {message}"))]
  TypeError { message: String },
  #[snafu(display("UnboundLocalError: cannot access local variable '{name}'"))]
  UnboundLocal { name: String },
  #[snafu(display("UnsupportedSyntax: {message}"))]
  UnsupportedSyntax { message: String },
}

impl From<ParseError> for Error {
  fn from(error: ParseError) -> Self {
    Self::Parse { source: error }
  }
}
