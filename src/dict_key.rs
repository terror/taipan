use super::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum DictKey {
  None,
  Number(NumberKey),
  Str(String),
  Tuple(Vec<DictKey>),
}

impl DictKey {
  pub(crate) fn new(object: &Object, heap: &Heap) -> Result<Self> {
    match object {
      Object::Bool(value) => {
        Ok(Self::Number(NumberKey::Int(i64::from(*value))))
      }
      Object::Float(value) => Ok(Self::Number(NumberKey::float(*value))),
      Object::Int(value) => Ok(Self::Number(NumberKey::Int(*value))),
      Object::None => Ok(Self::None),
      Object::Str(value) => Ok(Self::Str(value.clone())),
      Object::Tuple(tuple) => heap
        .tuple_ref(*tuple)?
        .iter()
        .map(|object| Self::new(object, heap))
        .collect::<Result<Vec<_>>>()
        .map(Self::Tuple),
      _ => Err(Error::TypeError {
        message: format!("unhashable type: '{}'", object.type_name()),
      }),
    }
  }
}
