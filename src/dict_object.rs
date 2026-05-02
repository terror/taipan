use super::*;

pub(crate) type DictObject = IndexMap<DictKey, DictObjectEntry>;

#[derive(Debug)]
pub(crate) struct DictObjectEntry {
  pub(crate) key: Object,
  pub(crate) value: Object,
}
