use super::*;

pub(crate) const BUILTINS: &[Builtin] = &[
  Builtin::Function(Function::new("int", int)),
  Builtin::Function(Function::new("len", len)),
  Builtin::Function(Function::new("print", print)),
  Builtin::Function(Function::new("str", str)),
  Builtin::Function(Function::new("type", r#type)),
];

fn int(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "int() takes exactly one argument".into(),
    });
  }

  match &arguments[0] {
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
        arguments[0].type_name()
      ),
    }),
  }
}

fn len(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "len() takes exactly one argument".into(),
    });
  }

  match &arguments[0] {
    Object::Str(s) => i64::try_from(s.len())
      .map(Object::Int)
      .map_err(|_| Error::Overflow),
    _ => Err(Error::TypeError {
      message: format!(
        "object of type '{}' has no len()",
        arguments[0].type_name()
      ),
    }),
  }
}

fn print(arguments: &[Object], output: &mut dyn Write) -> Result<Object> {
  writeln!(
    output,
    "{}",
    arguments
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>()
      .join(" ")
  )
  .map_err(|source| Error::Io { source })?;

  Ok(Object::None)
}

fn r#type(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "type() takes exactly one argument".into(),
    });
  }

  Ok(Object::Str(format!(
    "<class '{}'>",
    arguments[0].type_name()
  )))
}

fn str(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "str() takes exactly one argument".into(),
    });
  }

  Ok(Object::Str(arguments[0].to_string()))
}
