#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KernelSource {
  Environment,
  JupyterPath,
  System,
  User,
}

impl KernelSource {
  pub(crate) fn label(self) -> &'static str {
    match self {
      Self::Environment => "Environment",
      Self::JupyterPath => "JUPYTER_PATH",
      Self::System => "System",
      Self::User => "User",
    }
  }
}
