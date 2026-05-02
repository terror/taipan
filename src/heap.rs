use super::*;

#[derive(Debug, Default)]
pub struct Heap {
  pub(crate) objects: Vec<HeapObject>,
}

impl Heap {
  fn allocate(&mut self, object: HeapObject) -> usize {
    let reference = self.objects.len();

    self.objects.push(object);

    reference
  }

  pub(crate) fn dict_mut(
    &mut self,
    reference: DictRef,
  ) -> Result<&mut DictObject> {
    match self.objects.get_mut(reference.0) {
      Some(HeapObject::Dict(dict)) => Ok(dict),
      _ => Err(Error::Internal {
        message: "invalid dict reference".into(),
      }),
    }
  }

  pub(crate) fn dict_object(
    &mut self,
    entries: Vec<(Object, Object)>,
  ) -> Result<Object> {
    let mut result = DictObject::new();

    for (key, value) in entries {
      let dict_key = DictKey::new(&key, self)?;

      result
        .entry(dict_key)
        .and_modify(|entry| entry.value = value.clone())
        .or_insert(DictObjectEntry { key, value });
    }

    Ok(Object::Dict(DictRef(
      self.allocate(HeapObject::Dict(result)),
    )))
  }

  pub(crate) fn dict_ref(&self, reference: DictRef) -> Result<&DictObject> {
    match self.objects.get(reference.0) {
      Some(HeapObject::Dict(dict)) => Ok(dict),
      _ => Err(Error::Internal {
        message: "invalid dict reference".into(),
      }),
    }
  }

  pub(crate) fn iterator(&mut self, iterator: Iterator) -> Object {
    Object::Iterator(IteratorRef(self.allocate(HeapObject::Iterator(iterator))))
  }

  pub(crate) fn iterator_mut(
    &mut self,
    reference: IteratorRef,
  ) -> Result<&mut Iterator> {
    match self.objects.get_mut(reference.0) {
      Some(HeapObject::Iterator(iterator)) => Ok(iterator),
      _ => Err(Error::Internal {
        message: "invalid iterator reference".into(),
      }),
    }
  }

  pub(crate) fn list(&mut self, elements: Vec<Object>) -> Object {
    Object::List(ListRef(self.allocate(HeapObject::List(elements))))
  }

  pub(crate) fn list_mut(
    &mut self,
    reference: ListRef,
  ) -> Result<&mut Vec<Object>> {
    match self.objects.get_mut(reference.0) {
      Some(HeapObject::List(list)) => Ok(list),
      _ => Err(Error::Internal {
        message: "invalid list reference".into(),
      }),
    }
  }

  pub(crate) fn list_ref(&self, reference: ListRef) -> Result<&[Object]> {
    match self.objects.get(reference.0) {
      Some(HeapObject::List(list)) => Ok(list),
      _ => Err(Error::Internal {
        message: "invalid list reference".into(),
      }),
    }
  }

  pub(crate) fn tuple(&mut self, elements: Vec<Object>) -> Object {
    Object::Tuple(TupleRef(self.allocate(HeapObject::Tuple(elements))))
  }

  pub(crate) fn tuple_ref(&self, reference: TupleRef) -> Result<&[Object]> {
    match self.objects.get(reference.0) {
      Some(HeapObject::Tuple(tuple)) => Ok(tuple),
      _ => Err(Error::Internal {
        message: "invalid tuple reference".into(),
      }),
    }
  }
}
