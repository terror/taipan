#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DictRef(pub(crate) usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IteratorRef(pub(crate) usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ListRef(pub(crate) usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TupleRef(pub(crate) usize);
