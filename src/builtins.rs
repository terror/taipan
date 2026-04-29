use super::*;

pub(crate) const BUILTINS: &[Builtin] = &[
  Builtin::Function(Function::new("abs", abs)),
  Builtin::Function(Function::new("bool", bool)),
  Builtin::Function(Function::new("float", float)),
  Builtin::Function(Function::new("int", int)),
  Builtin::Function(Function::new("len", len)),
  Builtin::Function(Function::new("max", max)),
  Builtin::Function(Function::new("min", min)),
  Builtin::Function(Function::new("print", print)),
  Builtin::Function(Function::new("repr", repr)),
  Builtin::Function(Function::new("str", str)),
  Builtin::Function(Function::new("type", r#type)),
];

fn abs(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "abs() takes exactly one argument".into(),
    });
  }

  match &arguments[0] {
    Object::Int(integer) => integer
      .checked_abs()
      .map(Object::Int)
      .ok_or(Error::Overflow),
    Object::Float(float) => Ok(Object::Float(float.abs())),
    _ => Err(Error::TypeError {
      message: format!(
        "bad operand type for abs(): '{}'",
        arguments[0].type_name()
      ),
    }),
  }
}

fn bool(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "bool() takes exactly one argument".into(),
    });
  }

  Ok(Object::Bool(arguments[0].is_truthy()))
}

fn float(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "float() takes exactly one argument".into(),
    });
  }

  match &arguments[0] {
    Object::Int(integer) => {
      integer.to_f64().map(Object::Float).ok_or(Error::Overflow)
    }
    Object::Float(float) => Ok(Object::Float(*float)),
    Object::Bool(boolean) => Ok(Object::Float(f64::from(*boolean))),
    Object::Str(string) => {
      string
        .parse::<f64>()
        .map(Object::Float)
        .map_err(|_| Error::TypeError {
          message: format!("could not convert string to float: '{string}'"),
        })
    }
    _ => Err(Error::TypeError {
      message: format!(
        "float() argument must be a string or a number, not '{}'",
        arguments[0].type_name()
      ),
    }),
  }
}

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

fn max(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  minmax(arguments, Object::compare_gt, "max")
}

fn min(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  minmax(arguments, Object::compare_lt, "min")
}

fn minmax(
  arguments: &[Object],
  compare: fn(&Object, &Object) -> Result<Object>,
  name: &str,
) -> Result<Object> {
  if arguments.is_empty() {
    return Err(Error::TypeError {
      message: format!("{name}() expected at least one argument"),
    });
  }

  let mut result = &arguments[0];

  for argument in &arguments[1..] {
    if compare(argument, result)? == Object::Bool(true) {
      result = argument;
    }
  }

  Ok(result.clone())
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

fn repr(arguments: &[Object], _output: &mut dyn Write) -> Result<Object> {
  if arguments.len() != 1 {
    return Err(Error::TypeError {
      message: "repr() takes exactly one argument".into(),
    });
  }

  Ok(Object::Str(arguments[0].to_string()))
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
