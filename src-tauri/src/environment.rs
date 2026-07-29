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

  pub(super) fn search_roots(&self) -> Vec<SearchRoot> {
    let mut roots = self
      .paths("JUPYTER_PATH")
      .into_iter()
      .map(|path| SearchRoot {
        path: path.join("kernels"),
        source: KernelSource::JupyterPath,
      })
      .collect::<Vec<_>>();

    let user = self
      .path("JUPYTER_DATA_DIR")
      .or_else(|| self.user_data_dir())
      .map(|path| SearchRoot {
        path: path.join("kernels"),
        source: KernelSource::User,
      });

    let mut environments = ["VIRTUAL_ENV", "CONDA_PREFIX"]
      .into_iter()
      .filter_map(|name| self.path(name))
      .map(|path| SearchRoot {
        path: path.join("share").join("jupyter").join("kernels"),
        source: KernelSource::Environment,
      })
      .collect::<Vec<_>>();

    environments.dedup_by(|left, right| left.path == right.path);

    if self.truthy("JUPYTER_PREFER_ENV_PATH") {
      roots.append(&mut environments);
      roots.extend(user);
    } else {
      roots.extend(user);
      roots.append(&mut environments);
    }

    roots.extend(self.system_data_dirs().into_iter().map(|path| SearchRoot {
      path: path.join("kernels"),
      source: KernelSource::System,
    }));

    let mut paths = BTreeSet::new();
    roots.retain(|root| paths.insert(root.path.clone()));
    roots
  }

  fn system_data_dirs(&self) -> Vec<PathBuf> {
    match self.platform {
      Platform::Linux | Platform::Macos => vec![
        PathBuf::from("/usr/local/share/jupyter"),
        PathBuf::from("/usr/share/jupyter"),
      ],
      Platform::Windows => self
        .path("PROGRAMDATA")
        .map(|path| vec![path.join("jupyter")])
        .unwrap_or_default(),
    }
  }

  pub(crate) fn truthy(&self, name: &str) -> bool {
    self.variables.get(OsStr::new(name)).is_some_and(|value| {
      !matches!(
        value.to_string_lossy().to_ascii_lowercase().as_str(),
        "no" | "n" | "false" | "off" | "0" | "0.0"
      )
    })
  }

  fn user_data_dir(&self) -> Option<PathBuf> {
    match self.platform {
      Platform::Linux => self.path("XDG_DATA_HOME").or_else(|| {
        self
          .home
          .as_ref()
          .map(|home| home.join(".local").join("share"))
      }),
      Platform::Macos if self.truthy("JUPYTER_PLATFORM_DIRS") => self
        .home
        .as_ref()
        .map(|home| home.join("Library").join("Application Support")),
      Platform::Macos => self.home.as_ref().map(|home| home.join("Library")),
      Platform::Windows => self.path("APPDATA").or_else(|| {
        self
          .home
          .as_ref()
          .map(|home| home.join(".jupyter").join("data"))
      }),
    }
    .map(|path| {
      path.join(if self.platform == Platform::Macos {
        "Jupyter"
      } else {
        "jupyter"
      })
    })
  }
}
