use super::*;

#[derive(Clone, Debug, Default)]
pub enum Object {
  Bool(bool),
  Builtin(Builtin),
  Float(f64),
  Function {
    closure: Vec<Cell>,
    code: Rc<Code>,
    name: String,
    parameters: Vec<String>,
  },
  Int(i64),
  #[default]
  None,
  Str(String),
}

impl Object {
  pub(crate) fn binary_add(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        a.checked_add(*b).map(Self::Int).ok_or(Error::Overflow)
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a + b)),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float(int_to_float(*a)? + b)),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a + int_to_float(*b)?)),
      (Self::Str(a), Self::Str(b)) => Ok(Self::Str(format!("{a}{b}"))),
      _ => Err(self.binary_type_error("+", rhs)),
    }
  }

  pub(crate) fn binary_div(&self, rhs: &Self) -> Result<Self> {
    let (a, b) = match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => (int_to_float(*a)?, int_to_float(*b)?),
      (Self::Float(a), Self::Float(b)) => (*a, *b),
      (Self::Int(a), Self::Float(b)) => (int_to_float(*a)?, *b),
      (Self::Float(a), Self::Int(b)) => (*a, int_to_float(*b)?),
      _ => return Err(self.binary_type_error("/", rhs)),
    };

    if b == 0.0 {
      return Err(Error::TypeError {
        message: "division by zero".into(),
      });
    }

    Ok(Self::Float(a / b))
  }

  pub(crate) fn binary_floor_div(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Int(a.div_euclid(*b)))
      }
      (Self::Float(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Float((a / b).floor()))
      }
      (Self::Int(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Float((int_to_float(*a)? / b).floor()))
      }
      (Self::Float(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Float((a / int_to_float(*b)?).floor()))
      }
      _ => Err(self.binary_type_error("//", rhs)),
    }
  }

  pub(crate) fn binary_mod(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Int(a.rem_euclid(*b)))
      }
      (Self::Float(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Float(a % b))
      }
      (Self::Int(a), Self::Float(b)) => {
        if *b == 0.0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Float(int_to_float(*a)? % b))
      }
      (Self::Float(a), Self::Int(b)) => {
        if *b == 0 {
          return Err(Error::TypeError {
            message: "integer division or modulo by zero".into(),
          });
        }

        Ok(Self::Float(a % int_to_float(*b)?))
      }
      _ => Err(self.binary_type_error("%", rhs)),
    }
  }

  pub(crate) fn binary_mul(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        a.checked_mul(*b).map(Self::Int).ok_or(Error::Overflow)
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a * b)),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float(int_to_float(*a)? * b)),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a * int_to_float(*b)?)),
      (Self::Str(string), Self::Int(count))
      | (Self::Int(count), Self::Str(string)) => {
        let count = if *count <= 0 {
          0
        } else {
          usize::try_from(*count).map_err(|_| Error::Overflow)?
        };

        if string.is_empty() || count == 0 {
          return Ok(Self::Str(String::new()));
        }

        let capacity =
          string.len().checked_mul(count).ok_or(Error::Overflow)?;

        let mut result = String::new();

        result
          .try_reserve_exact(capacity)
          .map_err(|_| Error::Overflow)?;

        for _ in 0..count {
          result.push_str(string);
        }

        Ok(Self::Str(result))
      }
      _ => Err(self.binary_type_error("*", rhs)),
    }
  }

  pub(crate) fn binary_pow(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        if *b < 0 {
          Ok(Self::Float(int_to_float(*a)?.powi(pow_exponent(*b)?)))
        } else {
          a.checked_pow(u32::try_from(*b).map_err(|_| Error::Overflow)?)
            .map(Self::Int)
            .ok_or(Error::Overflow)
        }
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a.powf(*b))),
      (Self::Int(a), Self::Float(b)) => {
        Ok(Self::Float(int_to_float(*a)?.powf(*b)))
      }
      (Self::Float(a), Self::Int(b)) => {
        Ok(Self::Float(a.powi(pow_exponent(*b)?)))
      }
      _ => Err(self.binary_type_error("**", rhs)),
    }
  }

  pub(crate) fn binary_sub(&self, rhs: &Self) -> Result<Self> {
    match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => {
        a.checked_sub(*b).map(Self::Int).ok_or(Error::Overflow)
      }
      (Self::Float(a), Self::Float(b)) => Ok(Self::Float(a - b)),
      (Self::Int(a), Self::Float(b)) => Ok(Self::Float(int_to_float(*a)? - b)),
      (Self::Float(a), Self::Int(b)) => Ok(Self::Float(a - int_to_float(*b)?)),
      _ => Err(self.binary_type_error("-", rhs)),
    }
  }

  fn binary_type_error(&self, operator: &str, rhs: &Self) -> Error {
    Error::TypeError {
      message: format!(
        "unsupported operand type(s) for {operator}: '{}' and '{}'",
        self.type_name(),
        rhs.type_name()
      ),
    }
  }

  pub(crate) fn compare_eq(&self, rhs: &Self) -> Self {
    Self::Bool(self == rhs)
  }

  pub(crate) fn compare_ge(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, ">=", |a, b| a >= b)
  }

  pub(crate) fn compare_gt(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, ">", |a, b| a > b)
  }

  pub(crate) fn compare_le(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, "<=", |a, b| a <= b)
  }

  pub(crate) fn compare_lt(&self, rhs: &Self) -> Result<Self> {
    self.compare_numeric(rhs, "<", |a, b| a < b)
  }

  pub(crate) fn compare_ne(&self, rhs: &Self) -> Self {
    Self::Bool(self != rhs)
  }

  fn compare_numeric(
    &self,
    rhs: &Self,
    operator: &str,
    cmp: fn(f64, f64) -> bool,
  ) -> Result<Self> {
    let (a, b) = match (self, rhs) {
      (Self::Int(a), Self::Int(b)) => (int_to_float(*a)?, int_to_float(*b)?),
      (Self::Float(a), Self::Float(b)) => (*a, *b),
      (Self::Int(a), Self::Float(b)) => (int_to_float(*a)?, *b),
      (Self::Float(a), Self::Int(b)) => (*a, int_to_float(*b)?),
      (Self::Str(a), Self::Str(b)) => {
        return Ok(Self::Bool(cmp_str(a, b, operator)));
      }
      _ => {
        return Err(Error::TypeError {
          message: format!(
            "'{operator}' not supported between instances of '{}' and '{}'",
            self.type_name(),
            rhs.type_name()
          ),
        });
      }
    };

    Ok(Self::Bool(cmp(a, b)))
  }

  pub(crate) fn is_truthy(&self) -> bool {
    match self {
      Self::Bool(b) => *b,
      Self::Builtin(_) | Self::Function { .. } => true,
      Self::Float(f) => *f != 0.0,
      Self::Int(i) => *i != 0,
      Self::None => false,
      Self::Str(s) => !s.is_empty(),
    }
  }

  pub(crate) fn type_name(&self) -> &'static str {
    match self {
      Self::Bool(_) => "bool",
      Self::Builtin(_) => "builtin_function_or_method",
      Self::Float(_) => "float",
      Self::Function { .. } => "function",
      Self::Int(_) => "int",
      Self::None => "NoneType",
      Self::Str(_) => "str",
    }
  }

  pub(crate) fn unary_neg(&self) -> Result<Self> {
    match self {
      Self::Int(a) => a.checked_neg().map(Self::Int).ok_or(Error::Overflow),
      Self::Float(a) => Ok(Self::Float(-a)),
      _ => Err(Error::TypeError {
        message: format!(
          "bad operand type for unary -: '{}'",
          self.type_name()
        ),
      }),
    }
  }

  pub(crate) fn unary_not(&self) -> Self {
    Self::Bool(!self.is_truthy())
  }

  pub(crate) fn unary_pos(&self) -> Result<Self> {
    match self {
      Self::Int(a) => Ok(Self::Int(*a)),
      Self::Float(a) => Ok(Self::Float(*a)),
      _ => Err(Error::TypeError {
        message: format!(
          "bad operand type for unary +: '{}'",
          self.type_name()
        ),
      }),
    }
  }
}

fn cmp_str(a: &str, b: &str, operator: &str) -> bool {
  match operator {
    "<" => a < b,
    "<=" => a <= b,
    ">" => a > b,
    ">=" => a >= b,
    _ => unreachable!(),
  }
}

fn int_to_float(int: i64) -> Result<f64> {
  int.to_f64().ok_or(Error::Overflow)
}

fn pow_exponent(int: i64) -> Result<i32> {
  i32::try_from(int).map_err(|_| Error::Overflow)
}

impl Display for Object {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::Bool(true) => write!(f, "True"),
      Self::Bool(false) => write!(f, "False"),
      Self::Builtin(builtin) => {
        write!(f, "<built-in function {}>", builtin.name())
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
      (Self::Int(a), Self::Float(b)) => a.to_f64().is_some_and(|a| a == *b),
      (Self::Float(a), Self::Int(b)) => b.to_f64().is_some_and(|b| *a == b),
      (Self::Bool(a), Self::Bool(b)) => a == b,
      (
        Self::Function {
          closure: _,
          code: a_code,
          name: a_name,
          parameters: a_params,
        },
        Self::Function {
          closure: _,
          code: b_code,
          name: b_name,
          parameters: b_params,
        },
      ) => a_name == b_name && a_params == b_params && a_code == b_code,
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
    fn case(lhs: &Object, rhs: &Object, expected: &Object) {
      assert_eq!(&lhs.binary_add(rhs).unwrap(), expected);
    }

    case(&Object::Int(1), &Object::Int(2), &Object::Int(3));
    case(
      &Object::Float(1.5),
      &Object::Float(2.5),
      &Object::Float(4.0),
    );
    case(&Object::Int(1), &Object::Float(2.5), &Object::Float(3.5));
    case(
      &Object::Str("foo".into()),
      &Object::Str("bar".into()),
      &Object::Str("foobar".into()),
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
    #[track_caller]
    fn case(lhs: &Object, rhs: &Object, expected: &Object) {
      assert_eq!(&lhs.binary_mul(rhs).unwrap(), expected);
    }

    case(&Object::Int(3), &Object::Int(4), &Object::Int(12));
    case(
      &Object::Str("foo".into()),
      &Object::Int(3),
      &Object::Str("foofoofoo".into()),
    );
    case(
      &Object::Int(3),
      &Object::Str("foo".into()),
      &Object::Str("foofoofoo".into()),
    );
    case(
      &Object::Str("foo".into()),
      &Object::Int(0),
      &Object::Str(String::new()),
    );
    case(
      &Object::Str("foo".into()),
      &Object::Int(-1),
      &Object::Str(String::new()),
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
    fn case(obj: &Object, expected: &str) {
      assert_eq!(obj.to_string(), expected);
    }

    case(&Object::None, "None");
    case(&Object::Bool(true), "True");
    case(&Object::Bool(false), "False");
    case(&Object::Int(42), "42");
    case(&Object::Float(3.0), "3.0");
    case(&Object::Float(1.5), "1.5");
    case(&Object::Str("foo".into()), "foo");
  }

  #[test]
  fn truthiness() {
    #[track_caller]
    fn case(obj: &Object, expected: bool) {
      assert_eq!(obj.is_truthy(), expected);
    }

    case(&Object::None, false);
    case(&Object::Bool(false), false);
    case(&Object::Bool(true), true);
    case(&Object::Int(0), false);
    case(&Object::Int(1), true);
    case(&Object::Float(0.0), false);
    case(&Object::Float(0.1), true);
    case(&Object::Str(String::new()), false);
    case(&Object::Str("foo".into()), true);
  }
}
