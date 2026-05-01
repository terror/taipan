#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Symbol {
  Global,
  Local,
  Name,
  Nonlocal,
}
