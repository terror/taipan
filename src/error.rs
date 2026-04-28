use super::*;

#[derive(Debug)]
pub enum Error {
  Compile(String),
  NameError(String),
  Overflow,
  Parse(ParseError),
  TypeError(String),
  UnboundLocal(String),
  UnsupportedSyntax(String),
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::Compile(message) => write!(f, "CompileError: {message}"),
      Self::NameError(name) => {
        write!(f, "NameError: name '{name}' is not defined")
      }
      Self::Overflow => write!(f, "OverflowError: integer overflow"),
      Self::Parse(error) => write!(f, "SyntaxError: {error}"),
      Self::TypeError(message) => write!(f, "TypeError: {message}"),
      Self::UnboundLocal(name) => {
        write!(
          f,
          "UnboundLocalError: cannot access local variable '{name}'"
        )
      }
      Self::UnsupportedSyntax(message) => {
        write!(f, "UnsupportedSyntax: {message}")
      }
    }
  }
}

impl std::error::Error for Error {}

impl From<ParseError> for Error {
  fn from(error: ParseError) -> Self {
    Self::Parse(error)
  }
}
