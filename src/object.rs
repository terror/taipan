use super::*;

#[derive(Clone, Debug)]
pub struct BuiltinFn {
  pub func: fn(&[Object]) -> Result<Object>,
  pub name: &'static str,
}

#[derive(Clone, Debug, Default)]
pub enum Object {
  Bool(bool),
  BuiltinFn(BuiltinFn),
  Float(f64),
  Function {
    name: String,
    params: Vec<String>,
    code: Code,
  },
  Int(i64),
  #[default]
  None,
  Str(String),
}

impl Object {
  pub fn binary_add(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        a.checked_add(*b).map(Self::Int).ok_or(Error::Overflow)
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a + b)),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float(*a as f64 + b)),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a + *b as f64)),
      (Self::Str(a), Self::Str(b)) => Ok(Self::Str(format!("{a}{b}"))),
      _ => Err(self.binary_type_error("+", rhs)),
    }
  }

  pub fn binary_div(&self, rhs: &Self) -> Result<Self> {
    let (a, b) = match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => (*a as f64, *b as f64),
      (Self::Float(a), Self::Float(b)) => (*a, *b),
      (Self::Int(a), Self::Float(b)) => (*a as f64, *b),
      (Self::Float(a), Self::Int(b)) => (*a, *b as f64),
      _ => return Err(self.binary_type_error("/", rhs)),
    };

    if b == 0.0 {
      return Err(Error::TypeError("division by zero".into()));
    }

    Ok(Self::Float(a / b))
  }

  pub fn binary_floor_div(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Int(a.div_euclid(*b)))
      }
      (Self::Float(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Float((a / b).floor()))
      }
      (Self::Int(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Float((*a as f64 / b).floor()))
      }
      (Self::Float(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Float((a / *b as f64).floor()))
      }
      _ => Err(self.binary_type_error("//", rhs)),
    }
  }

  pub fn binary_mod(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Int(a.rem_euclid(*b)))
      }
      (Self::Float(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Float(a % b))
      }
      (Self::Int(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Float(*a as f64 % b))
      }
      (Self::Float(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError(
            "integer division or modulo by zero".into(),
          ));
        }

        Ok(Self::Float(a % *b as f64))
      }
      _ => Err(self.binary_type_error("%", rhs)),
    }
  }

  pub fn binary_mul(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        a.checked_mul(*b).map(Self::Int).ok_or(Error::Overflow)
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a * b)),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float(*a as f64 * b)),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a * *b as f64)),
      _ => Err(self.binary_type_error("*", rhs)),
    }
  }

  pub fn binary_pow(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        if *b < 0 {
          Ok(Self::Float((*a as f64).powi(*b as i32)))
        } else {
          a.checked_pow(u32::try_from(*b).map_err(|_| Error::Overflow)?)
            .map(Self::Int)
            .ok_or(Error::Overflow)
        }
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a.powf(*b))),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float((*a as f64).powf(*b))),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a.powi(*b as i32))),
      _ => Err(self.binary_type_error("**", rhs)),
    }
  }

  pub fn binary_sub(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        a.checked_sub(*b).map(Self::Int).ok_or(Error::Overflow)
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a - b)),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float(*a as f64 - b)),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a - *b as f64)),
      _ => Err(self.binary_type_error("-", rhs)),
    }
  }

  fn binary_type_error(&self, op: &str, rhs: &Self) -> Error {
    Error::TypeError(format!(
      "unsupported operand type(s) for {op}: '{}' and '{}'",
      self.type_name(),
      rhs.type_name()
    ))
  }

  pub fn compare_eq(&self, rhs: &Self) -> Self {
    Self::Bool(self == rhs)
  }

  pub fn compare_ge(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, ">=", |a, b| a >= b)
  }

  pub fn compare_gt(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, ">", |a, b| a > b)
  }

  pub fn compare_le(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, "<=", |a, b| a <= b)
  }

  pub fn compare_lt(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, "<", |a, b| a < b)
  }

  pub fn compare_ne(&self, rhs: &Self) -> Self {
    Self::Bool(self != rhs)
  }

  fn compare_numeric(
    &self,
    rhs: &Self,
    op: &str,
    cmp: fn(f64, f64) -> bool,
  ) -> Result<Self> {
    let (a, b) = match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => (*a as f64, *b as f64),
      (Self::Float(a), Self::Float(b)) => (*a, *b),
      (Self::Int(a), Self::Float(b)) => (*a as f64, *b),
      (Self::Float(a), Self::Int(b)) => (*a, *b as f64),
      (Self::Str(a), Self::Str(b)) => return Ok(Self::Bool(cmp_str(a, b, op))),
      _ => {
        return Err(Error::TypeError(format!(
          "'{op}' not supported between instances of '{}' and '{}'",
          self.type_name(),
          rhs.type_name()
        )));
      }
    };

    Ok(Self::Bool(cmp(a, b)))
  }

  pub fn is_truthy(&self) -> bool {
    match self {
      Self::Bool(b) => *b,
      Self::BuiltinFn(_) | Self::Function { .. } => true,
      Self::Float(f) => *f != 0.0,
      Self::Int(i) => *i != 0,
      Self::None => false,
      Self::Str(s) => !s.is_empty(),
    }
  }

  pub fn type_name(&self) -> &'static str {
    match self {
      Self::Bool(_) => "bool",
      Self::BuiltinFn(_) => "builtin_function_or_method",
      Self::Float(_) => "float",
      Self::Function { .. } => "function",
      Self::Int(_) => "int",
      Self::None => "NoneType",
      Self::Str(_) => "str",
    }
  }

  pub fn unary_neg(&self) -> Result<Self> {
    match self {
      Self::Int(a) => a.checked_neg().map(Self::Int).ok_or(Error::Overflow),
      Self::Float(a) => Ok(Self::Float(-a)),
      _ => Err(Error::TypeError(format!(
        "bad operand type for unary -: '{}'",
        self.type_name()
      ))),
    }
  }

  pub fn unary_not(&self) -> Self {
    Self::Bool(!self.is_truthy())
  }

  pub fn unary_pos(&self) -> Result<Self> {
    match self {
      Self::Int(a) => Ok(Self::Int(*a)),
      Self::Float(a) => Ok(Self::Float(*a)),
      _ => Err(Error::TypeError(format!(
        "bad operand type for unary +: '{}'",
        self.type_name()
      ))),
    }
  }
}

fn cmp_str(a: &str, b: &str, op: &str) -> bool {
  match op {
    "<" => a < b,
    "<=" => a <= b,
    ">" => a > b,
    ">=" => a >= b,
    _ => unreachable!(),
  }
}

impl Display for Object {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::Bool(true) => write!(f, "True"),
      Self::Bool(false) => write!(f, "False"),
      Self::BuiltinFn(function) => {
        write!(f, "<built-in function {}>", function.name)
      }
      Self::Float(float) => {
        if float.fract() == 0.0 && float.is_finite() {
          write!(f, "{float:.1}")
        } else {
          write!(f, "{float}")
        }
      }
      Self::Function { name, .. } => write!(f, "<function {name}>"),
      Self::Int(int) => write!(f, "{int}"),
      Self::None => write!(f, "None"),
      Self::Str(string) => write!(f, "{string}"),
    }
  }
}

impl PartialEq for Object {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Int(a), Self::Int(b)) => a == b,
      (Self::Float(a), Self::Float(b)) => a == b,
      (Self::Int(a), Self::Float(b)) => *a as f64 == *b,
      (Self::Float(a), Self::Int(b)) => *a == *b as f64,
      (Self::Bool(a), Self::Bool(b)) => a == b,
      (Self::Str(a), Self::Str(b)) => a == b,
      (Self::None, Self::None) => true,
      _ => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn binary_add() {
    #[track_caller]
    fn case(lhs: Object, rhs: Object, expected: Object) {
      assert_eq!(lhs.binary_add(&rhs).unwrap(), expected);
    }

    case(Object::Int(1), Object::Int(2), Object::Int(3));
    case(Object::Float(1.5), Object::Float(2.5), Object::Float(4.0));
    case(Object::Int(1), Object::Float(2.5), Object::Float(3.5));
    case(
      Object::Str("foo".into()),
      Object::Str("bar".into()),
      Object::Str("foobar".into()),
    );
  }

  #[test]
  fn binary_add_type_error() {
    assert!(
      Object::Int(1)
        .binary_add(&Object::Str("foo".into()))
        .is_err()
    );
  }

  #[test]
  fn binary_div() {
    assert_eq!(
      Object::Int(7).binary_div(&Object::Int(2)).unwrap(),
      Object::Float(3.5)
    );
  }

  #[test]
  fn binary_div_by_zero() {
    assert!(Object::Int(1).binary_div(&Object::Int(0)).is_err());
  }

  #[test]
  fn binary_floor_div() {
    assert_eq!(
      Object::Int(7).binary_floor_div(&Object::Int(2)).unwrap(),
      Object::Int(3)
    );
  }

  #[test]
  fn binary_mod() {
    assert_eq!(
      Object::Int(7).binary_mod(&Object::Int(3)).unwrap(),
      Object::Int(1)
    );
  }

  #[test]
  fn binary_mul() {
    assert_eq!(
      Object::Int(3).binary_mul(&Object::Int(4)).unwrap(),
      Object::Int(12)
    );
  }

  #[test]
  fn binary_pow() {
    assert_eq!(
      Object::Int(2).binary_pow(&Object::Int(10)).unwrap(),
      Object::Int(1024)
    );
  }

  #[test]
  fn binary_sub() {
    assert_eq!(
      Object::Int(5).binary_sub(&Object::Int(3)).unwrap(),
      Object::Int(2)
    );
  }

  #[test]
  fn comparison() {
    assert_eq!(
      Object::Int(1).compare_eq(&Object::Int(1)),
      Object::Bool(true)
    );

    assert_eq!(
      Object::Int(1).compare_ne(&Object::Int(2)),
      Object::Bool(true)
    );

    assert_eq!(
      Object::Int(1).compare_lt(&Object::Int(2)).unwrap(),
      Object::Bool(true)
    );

    assert_eq!(
      Object::Str("a".into())
        .compare_lt(&Object::Str("b".into()))
        .unwrap(),
      Object::Bool(true)
    );
  }

  #[test]
  fn display() {
    #[track_caller]
    fn case(obj: Object, expected: &str) {
      assert_eq!(obj.to_string(), expected);
    }

    case(Object::None, "None");
    case(Object::Bool(true), "True");
    case(Object::Bool(false), "False");
    case(Object::Int(42), "42");
    case(Object::Float(3.0), "3.0");
    case(Object::Float(1.5), "1.5");
    case(Object::Str("foo".into()), "foo");
  }

  #[test]
  fn truthiness() {
    #[track_caller]
    fn case(obj: Object, expected: bool) {
      assert_eq!(obj.is_truthy(), expected);
    }

    case(Object::None, false);
    case(Object::Bool(false), false);
    case(Object::Bool(true), true);
    case(Object::Int(0), false);
    case(Object::Int(1), true);
    case(Object::Float(0.0), false);
    case(Object::Float(0.1), true);
    case(Object::Str(String::new()), false);
    case(Object::Str("foo".into()), true);
  }
}
