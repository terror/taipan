use super::*;

#[derive(Clone, Debug)]
pub struct Iterator {
  index: usize,
  items: Vec<Object>,
}

impl Iterator {
  pub(crate) fn new(items: Vec<Object>) -> Self {
    Self { index: 0, items }
  }
}

impl std::iter::Iterator for Iterator {
  type Item = Object;

  fn next(&mut self) -> Option<Self::Item> {
    let item = self.items.get(self.index).cloned();

    self.index += usize::from(item.is_some());

    item
  }
}
