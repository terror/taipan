use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchRoot {
  pub(crate) path: PathBuf,
  pub(crate) source: KernelSource,
}
