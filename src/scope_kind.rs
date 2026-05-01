#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ScopeKind {
  Function,
  #[default]
  Module,
}
