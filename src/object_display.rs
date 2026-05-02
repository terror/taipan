use super::*;

pub struct ObjectDisplay<'a> {
  heap: &'a Heap,
  object: &'a Object,
}

impl<'a> ObjectDisplay<'a> {
  pub(crate) fn new(object: &'a Object, heap: &'a Heap) -> Self {
    Self { heap, object }
  }
}

impl Display for ObjectDisplay<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self.object {
      Object::Bool(true) => write!(f, "True"),
      Object::Bool(false) => write!(f, "False"),
      Object::Builtin(builtin) => {
        write!(f, "<built-in function {}>", builtin.name())
      }
      Object::Dict(dict) => {
        let dict = self.heap.dict_ref(*dict).map_err(|_| fmt::Error)?;

        write!(f, "{{")?;

        for (index, entry) in dict.values().enumerate() {
          if index > 0 {
            write!(f, ", ")?;
          }

          write!(
            f,
            "{}: {}",
            ObjectDisplay::new(&entry.key, self.heap),
            ObjectDisplay::new(&entry.value, self.heap)
          )?;
        }

        write!(f, "}}")
      }
      Object::Float(float) => {
        if float.fract() == 0.0 && float.is_finite() {
          write!(f, "{float:.1}")
        } else {
          write!(f, "{float}")
        }
      }
      Object::Function { name, .. } => write!(f, "<function {name}>"),
      Object::Int(int) => write!(f, "{int}"),
      Object::Iterator(_) => write!(f, "<iterator>"),
      Object::List(list) => {
        let list = self.heap.list_ref(*list).map_err(|_| fmt::Error)?;

        write!(f, "[")?;

        for (index, object) in list.iter().enumerate() {
          if index > 0 {
            write!(f, ", ")?;
          }

          write!(f, "{}", ObjectDisplay::new(object, self.heap))?;
        }

        write!(f, "]")
      }
      Object::None => write!(f, "None"),
      Object::Str(string) => write!(f, "{string}"),
      Object::Tuple(tuple) => {
        let tuple = self.heap.tuple_ref(*tuple).map_err(|_| fmt::Error)?;

        write!(f, "(")?;

        for (index, object) in tuple.iter().enumerate() {
          if index > 0 {
            write!(f, ", ")?;
          }

          write!(f, "{}", ObjectDisplay::new(object, self.heap))?;
        }

        if tuple.len() == 1 {
          write!(f, ",")?;
        }

        write!(f, ")")
      }
    }
  }
}
