use super::*;

#[derive(Debug)]
pub(crate) enum HeapObject {
  Dict(DictObject),
  Iterator(Iterator),
  List(Vec<Object>),
  Tuple(Vec<Object>),
}
