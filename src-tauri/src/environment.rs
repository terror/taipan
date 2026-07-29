use super::*;

pub(crate) struct Environment {
  pub(crate) home: Option<PathBuf>,
  pub(crate) platform: Platform,
  pub(crate) variables: BTreeMap<OsString, OsString>,
}

impl Environment {
  pub(crate) fn current() -> Self {
    #[cfg(target_os = "macos")]
    let platform = Platform::Macos;
    #[cfg(target_os = "windows")]
    let platform = Platform::Windows;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let platform = Platform::Linux;

    let variables = std::env::vars_os().collect::<BTreeMap<_, _>>();

    let home = variables
      .get(OsStr::new(if platform == Platform::Windows {
        "USERPROFILE"
      } else {
        "HOME"
      }))
      .filter(|value| !value.is_empty())
      .map(PathBuf::from);

    Self {
      home,
      platform,
      variables,
    }
  }

  pub(crate) fn path(&self, name: &str) -> Option<PathBuf> {
    self
      .variables
      .get(OsStr::new(name))
      .filter(|value| !value.is_empty())
      .map(PathBuf::from)
  }

  pub(crate) fn paths(&self, name: &str) -> Vec<PathBuf> {
    self
      .variables
      .get(OsStr::new(name))
      .filter(|value| !value.is_empty())
      .map(|value| {
        std::env::split_paths(value)
          .filter(|path| !path.as_os_str().is_empty())
          .collect()
      })
      .unwrap_or_default()
  }

  pub(crate) fn truthy(&self, name: &str) -> bool {
    self.variables.get(OsStr::new(name)).is_some_and(|value| {
      !matches!(
        value.to_string_lossy().to_ascii_lowercase().as_str(),
        "no" | "n" | "false" | "off" | "0" | "0.0"
      )
    })
  }
}
