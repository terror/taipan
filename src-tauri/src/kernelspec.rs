use {
  super::*,
  crate::{kernel::KernelLaunchSpec, notebook::Metadata},
};

const CONNECTION_FILE: &str = "{connection_file}";

#[typeshare]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KernelDiagnostic {
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  pub source: String,
}

#[typeshare]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KernelDiscovery {
  pub diagnostics: Vec<KernelDiagnostic>,
  pub kernels: Vec<KernelSummary>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub recommended_id: Option<String>,
}

#[typeshare]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KernelSummary {
  pub display_name: String,
  pub id: String,
  pub language: String,
  pub name: String,
  pub source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KernelSource {
  Environment,
  JupyterPath,
  System,
  User,
}

impl KernelSource {
  fn label(self) -> &'static str {
    match self {
      Self::Environment => "Environment",
      Self::JupyterPath => "JUPYTER_PATH",
      Self::System => "System",
      Self::User => "User",
    }
  }
}

#[derive(Debug, Deserialize)]
struct KernelJson {
  argv: Vec<String>,
  display_name: String,
  #[serde(default)]
  env: BTreeMap<String, String>,
  language: String,
}

#[allow(dead_code)]
#[derive(Debug)]
struct KernelSpec {
  argv: Vec<String>,
  display_name: String,
  env: BTreeMap<String, String>,
  kernel_file: PathBuf,
  language: String,
  name: String,
  resource_dir: PathBuf,
  source: KernelSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SearchRoot {
  path: PathBuf,
  source: KernelSource,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Platform {
  Linux,
  Macos,
  Windows,
}

struct SearchEnvironment {
  home: Option<PathBuf>,
  platform: Platform,
  variables: BTreeMap<OsString, OsString>,
}

impl SearchEnvironment {
  fn current() -> Self {
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

  fn path(&self, name: &str) -> Option<PathBuf> {
    self
      .variables
      .get(OsStr::new(name))
      .filter(|value| !value.is_empty())
      .map(PathBuf::from)
  }

  fn paths(&self, name: &str) -> Vec<PathBuf> {
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

  fn truthy(&self, name: &str) -> bool {
    self.variables.get(OsStr::new(name)).is_some_and(|value| {
      !matches!(
        value.to_string_lossy().to_ascii_lowercase().as_str(),
        "no" | "n" | "false" | "off" | "0" | "0.0"
      )
    })
  }
}

pub struct KernelSpecManager;

impl KernelSpecManager {
  pub fn discover(metadata: &Metadata) -> KernelDiscovery {
    Self::discover_in(&search_roots(&SearchEnvironment::current()), metadata)
  }

  fn discover_in(roots: &[SearchRoot], metadata: &Metadata) -> KernelDiscovery {
    let (specs, diagnostics) = load_specs(roots);
    let recommended = recommendation(&specs, metadata);
    let kernels = specs
      .iter()
      .enumerate()
      .map(|(index, spec)| KernelSummary {
        display_name: spec.display_name.clone(),
        id: format!("kernel-{index}"),
        language: spec.language.clone(),
        name: spec.name.clone(),
        source: spec.source.label().into(),
      })
      .collect::<Vec<_>>();
    let recommended_id = recommended.map(|index| kernels[index].id.clone());

    KernelDiscovery {
      diagnostics,
      kernels,
      recommended_id,
    }
  }

  pub fn launch_spec(
    name: &str,
  ) -> std::result::Result<KernelLaunchSpec, String> {
    let (specs, _) = load_specs(&search_roots(&SearchEnvironment::current()));
    let spec = specs
      .into_iter()
      .find(|spec| spec.name.eq_ignore_ascii_case(name))
      .ok_or_else(|| format!("kernelspec `{name}` is not available"))?;

    Ok(KernelLaunchSpec {
      argv: spec.argv,
      env: spec.env,
      language: spec.language,
      resource_dir: Some(spec.resource_dir),
    })
  }
}

fn load_specs(
  roots: &[SearchRoot],
) -> (Vec<KernelSpec>, Vec<KernelDiagnostic>) {
  let mut diagnostics = Vec::new();
  let mut names = BTreeSet::new();
  let mut specs = Vec::new();

  for root in roots {
    let entries = match fs::read_dir(&root.path) {
      Ok(entries) => entries,
      Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
      Err(error) => {
        diagnostics.push(KernelDiagnostic {
          message: format!("failed to read kernelspec directory: {error}"),
          name: None,
          source: root.source.label().into(),
        });
        continue;
      }
    };

    let mut entries = entries.collect::<Vec<_>>();
    entries.sort_by(|left, right| match (left, right) {
      (Ok(left), Ok(right)) => left
        .file_name()
        .to_string_lossy()
        .to_ascii_lowercase()
        .cmp(&right.file_name().to_string_lossy().to_ascii_lowercase())
        .then_with(|| left.file_name().cmp(&right.file_name())),
      (Ok(_), Err(_)) => std::cmp::Ordering::Less,
      (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
      (Err(left), Err(right)) => left.to_string().cmp(&right.to_string()),
    });

    for entry in entries {
      let entry = match entry {
        Ok(entry) => entry,
        Err(error) => {
          diagnostics.push(KernelDiagnostic {
            message: format!("failed to read kernelspec entry: {error}"),
            name: None,
            source: root.source.label().into(),
          });
          continue;
        }
      };

      let resource_dir = entry.path();
      let kernel_file = resource_dir.join("kernel.json");

      if !resource_dir.is_dir() || !kernel_file.is_file() {
        continue;
      }

      let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
        diagnostics.push(KernelDiagnostic {
          message: "kernelspec directory name is not valid UTF-8".into(),
          name: None,
          source: root.source.label().into(),
        });
        continue;
      };

      if !valid_name(&name) {
        diagnostics.push(KernelDiagnostic {
            message: "name must contain only ASCII letters, numbers, hyphens, periods, and underscores".into(),
            name: Some(name),
            source: root.source.label().into(),
          });
        continue;
      }

      let canonical_name = name.to_ascii_lowercase();

      if names.contains(&canonical_name) {
        continue;
      }

      match load_spec(
        canonical_name.clone(),
        root.source,
        resource_dir,
        kernel_file,
      ) {
        Ok(spec) => {
          names.insert(canonical_name);
          specs.push(spec);
        }
        Err(message) => diagnostics.push(KernelDiagnostic {
          message,
          name: Some(name),
          source: root.source.label().into(),
        }),
      }
    }
  }

  (specs, diagnostics)
}

fn load_spec(
  name: String,
  source: KernelSource,
  resource_dir: PathBuf,
  kernel_file: PathBuf,
) -> std::result::Result<KernelSpec, String> {
  let bytes = fs::read(&kernel_file)
    .map_err(|error| format!("failed to read kernel.json: {error}"))?;
  let json = serde_json::from_slice::<KernelJson>(&bytes)
    .map_err(|error| format!("invalid kernel.json: {error}"))?;

  if json.argv.is_empty() || json.argv[0].is_empty() {
    return Err("argv must contain a non-empty executable".into());
  }

  if !json
    .argv
    .iter()
    .any(|argument| argument.contains(CONNECTION_FILE))
  {
    return Err("argv must contain {connection_file}".into());
  }

  if json.display_name.trim().is_empty() {
    return Err("display_name must not be empty".into());
  }

  if json.language.trim().is_empty() {
    return Err("language must not be empty".into());
  }

  Ok(KernelSpec {
    argv: json.argv,
    display_name: json.display_name,
    env: json.env,
    kernel_file,
    language: json.language,
    name,
    resource_dir,
    source,
  })
}

fn metadata_string<'a>(
  metadata: &'a Metadata,
  object: &str,
  field: &str,
) -> Option<&'a str> {
  metadata
    .get(object)
    .and_then(Value::as_object)
    .and_then(|value| value.get(field))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
}

fn recommendation(specs: &[KernelSpec], metadata: &Metadata) -> Option<usize> {
  if let Some(name) = metadata_string(metadata, "kernelspec", "name")
    && let Some(index) = specs
      .iter()
      .position(|spec| spec.name.eq_ignore_ascii_case(name))
  {
    return Some(index);
  }

  [
    metadata_string(metadata, "kernelspec", "language"),
    metadata_string(metadata, "language_info", "name"),
  ]
  .into_iter()
  .flatten()
  .find_map(|language| {
    specs
      .iter()
      .position(|spec| spec.language.to_lowercase() == language.to_lowercase())
  })
}

fn search_roots(environment: &SearchEnvironment) -> Vec<SearchRoot> {
  let mut roots = environment
    .paths("JUPYTER_PATH")
    .into_iter()
    .map(|path| SearchRoot {
      path: path.join("kernels"),
      source: KernelSource::JupyterPath,
    })
    .collect::<Vec<_>>();

  let user = environment
    .path("JUPYTER_DATA_DIR")
    .or_else(|| user_data_dir(environment))
    .map(|path| SearchRoot {
      path: path.join("kernels"),
      source: KernelSource::User,
    });

  let mut environments = ["VIRTUAL_ENV", "CONDA_PREFIX"]
    .into_iter()
    .filter_map(|name| environment.path(name))
    .map(|path| SearchRoot {
      path: path.join("share").join("jupyter").join("kernels"),
      source: KernelSource::Environment,
    })
    .collect::<Vec<_>>();
  environments.dedup_by(|left, right| left.path == right.path);

  if environment.truthy("JUPYTER_PREFER_ENV_PATH") {
    roots.append(&mut environments);
    roots.extend(user);
  } else {
    roots.extend(user);
    roots.append(&mut environments);
  }

  roots.extend(system_data_dirs(environment).into_iter().map(|path| {
    SearchRoot {
      path: path.join("kernels"),
      source: KernelSource::System,
    }
  }));

  let mut paths = BTreeSet::new();
  roots.retain(|root| paths.insert(root.path.clone()));
  roots
}

fn system_data_dirs(environment: &SearchEnvironment) -> Vec<PathBuf> {
  match environment.platform {
    Platform::Linux | Platform::Macos => vec![
      PathBuf::from("/usr/local/share/jupyter"),
      PathBuf::from("/usr/share/jupyter"),
    ],
    Platform::Windows => environment
      .path("PROGRAMDATA")
      .map(|path| vec![path.join("jupyter")])
      .unwrap_or_default(),
  }
}

fn user_data_dir(environment: &SearchEnvironment) -> Option<PathBuf> {
  match environment.platform {
    Platform::Linux => environment.path("XDG_DATA_HOME").or_else(|| {
      environment
        .home
        .as_ref()
        .map(|home| home.join(".local").join("share"))
    }),
    Platform::Macos if environment.truthy("JUPYTER_PLATFORM_DIRS") => {
      environment
        .home
        .as_ref()
        .map(|home| home.join("Library").join("Application Support"))
    }
    Platform::Macos => {
      environment.home.as_ref().map(|home| home.join("Library"))
    }
    Platform::Windows => environment.path("APPDATA").or_else(|| {
      environment
        .home
        .as_ref()
        .map(|home| home.join(".jupyter").join("data"))
    }),
  }
  .map(|path| {
    path.join(if environment.platform == Platform::Macos {
      "Jupyter"
    } else {
      "jupyter"
    })
  })
}

fn valid_name(name: &str) -> bool {
  !name.is_empty()
    && name.bytes().all(|byte| {
      byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_')
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn environment(platform: Platform) -> SearchEnvironment {
    SearchEnvironment {
      home: Some(PathBuf::from("/home/foo")),
      platform,
      variables: BTreeMap::new(),
    }
  }

  fn root(path: &Path, source: KernelSource) -> SearchRoot {
    SearchRoot {
      path: path.into(),
      source,
    }
  }

  fn write_spec(root: &Path, name: &str, json: &Value) {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
      directory.join("kernel.json"),
      serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
  }

  fn valid_spec(display_name: &str, language: &str) -> Value {
    serde_json::json!({
      "argv": ["foo", "--connection", "{connection_file}"],
      "display_name": display_name,
      "env": {"FOO": "bar"},
      "language": language,
    })
  }

  #[test]
  fn search_path_overrides_and_precedence() {
    let mut environment = environment(Platform::Linux);
    environment.variables.extend([
      (
        OsString::from("JUPYTER_PATH"),
        std::env::join_paths(["/foo", "/bar"]).unwrap(),
      ),
      (OsString::from("JUPYTER_DATA_DIR"), OsString::from("/user")),
      (OsString::from("VIRTUAL_ENV"), OsString::from("/env")),
    ]);

    assert_eq!(
      search_roots(&environment),
      [
        root(Path::new("/foo/kernels"), KernelSource::JupyterPath),
        root(Path::new("/bar/kernels"), KernelSource::JupyterPath),
        root(Path::new("/user/kernels"), KernelSource::User),
        root(
          Path::new("/env/share/jupyter/kernels"),
          KernelSource::Environment,
        ),
        root(
          Path::new("/usr/local/share/jupyter/kernels"),
          KernelSource::System,
        ),
        root(
          Path::new("/usr/share/jupyter/kernels"),
          KernelSource::System,
        ),
      ]
    );

    environment.variables.insert(
      OsString::from("JUPYTER_PREFER_ENV_PATH"),
      OsString::from("1"),
    );
    let roots = search_roots(&environment);
    assert_eq!(roots[2].source, KernelSource::Environment);
    assert_eq!(roots[3].source, KernelSource::User);
  }

  #[test]
  fn platform_user_and_system_locations() {
    #[track_caller]
    fn case(environment: &SearchEnvironment, user: &str, system: &[&str]) {
      let roots = search_roots(environment);
      assert_eq!(roots[0].path, Path::new(user));
      assert_eq!(
        roots[1..]
          .iter()
          .map(|root| root.path.as_path())
          .collect::<Vec<_>>(),
        system.iter().map(Path::new).collect::<Vec<_>>()
      );
    }

    case(
      &environment(Platform::Linux),
      "/home/foo/.local/share/jupyter/kernels",
      &[
        "/usr/local/share/jupyter/kernels",
        "/usr/share/jupyter/kernels",
      ],
    );
    case(
      &environment(Platform::Macos),
      "/home/foo/Library/Jupyter/kernels",
      &[
        "/usr/local/share/jupyter/kernels",
        "/usr/share/jupyter/kernels",
      ],
    );

    let mut windows = environment(Platform::Windows);
    windows.variables.extend([
      (OsString::from("APPDATA"), OsString::from("C:/Users/foo")),
      (OsString::from("PROGRAMDATA"), OsString::from("C:/Data")),
    ]);
    case(
      &windows,
      "C:/Users/foo/jupyter/kernels",
      &["C:/Data/jupyter/kernels"],
    );
  }

  #[test]
  fn discovers_by_precedence_and_case_insensitive_name() {
    let high = tempfile::tempdir().unwrap();
    let low = tempfile::tempdir().unwrap();
    write_spec(high.path(), "Python3", &valid_spec("High", "python"));
    write_spec(low.path(), "python3", &valid_spec("Low", "python"));
    write_spec(low.path(), "julia", &valid_spec("Julia", "julia"));

    let discovery = KernelSpecManager::discover_in(
      &[
        root(high.path(), KernelSource::User),
        root(low.path(), KernelSource::System),
      ],
      &Metadata::new(),
    );

    assert_eq!(
      discovery
        .kernels
        .iter()
        .map(|kernel| (&kernel.name, &kernel.display_name, &kernel.source))
        .collect::<Vec<_>>(),
      [
        (&"python3".into(), &"High".into(), &"User".into()),
        (&"julia".into(), &"Julia".into(), &"System".into()),
      ]
    );
  }

  #[test]
  fn malformed_specs_produce_diagnostics_without_hiding_valid_specs() {
    let high = tempfile::tempdir().unwrap();
    let low = tempfile::tempdir().unwrap();
    write_spec(
      high.path(),
      "python3",
      &serde_json::json!({
        "argv": ["foo"],
        "display_name": "Python",
        "language": "python",
      }),
    );
    write_spec(high.path(), "invalid name", &valid_spec("Bad", "foo"));
    write_spec(low.path(), "PYTHON3", &valid_spec("Python", "python"));
    write_spec(low.path(), "julia", &valid_spec("Julia", "julia"));

    let discovery = KernelSpecManager::discover_in(
      &[
        root(high.path(), KernelSource::User),
        root(low.path(), KernelSource::System),
      ],
      &Metadata::new(),
    );

    assert_eq!(
      discovery
        .kernels
        .iter()
        .map(|kernel| kernel.name.as_str())
        .collect::<Vec<_>>(),
      ["julia", "python3"]
    );
    assert_eq!(discovery.diagnostics.len(), 2);
    assert!(
      discovery
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains(CONNECTION_FILE))
    );
  }

  #[test]
  fn validates_required_fields_argv_and_placeholder() {
    #[track_caller]
    fn case(json: &Value, expected: &str) {
      let directory = tempfile::tempdir().unwrap();
      write_spec(directory.path(), "foo", json);
      let discovery = KernelSpecManager::discover_in(
        &[root(directory.path(), KernelSource::User)],
        &Metadata::new(),
      );
      assert!(discovery.kernels.is_empty());
      assert!(discovery.diagnostics[0].message.contains(expected));
    }

    case(
      &serde_json::json!({"display_name": "Foo", "language": "foo"}),
      "missing field `argv`",
    );
    case(
      &serde_json::json!({
        "argv": [],
        "display_name": "Foo",
        "language": "foo",
      }),
      "non-empty executable",
    );
    case(
      &serde_json::json!({
        "argv": ["foo"],
        "display_name": "Foo",
        "language": "foo",
      }),
      CONNECTION_FILE,
    );
    case(
      &serde_json::json!({
        "argv": ["foo", "{connection_file}"],
        "display_name": 1,
        "language": "foo",
      }),
      "invalid type",
    );
    case(
      &serde_json::json!({
        "argv": ["foo", "{connection_file}"],
        "display_name": "Foo",
        "env": {"FOO": 1},
        "language": "foo",
      }),
      "invalid type",
    );
  }

  #[test]
  fn recommendation_prefers_name_then_language_metadata() {
    let directory = tempfile::tempdir().unwrap();
    write_spec(directory.path(), "Julia", &valid_spec("Julia", "julia"));
    write_spec(directory.path(), "python3", &valid_spec("Python", "python"));
    let roots = [root(directory.path(), KernelSource::User)];

    let mut metadata = Metadata::new();
    metadata.insert(
      "kernelspec".into(),
      serde_json::json!({"name": "PYTHON3", "language": "julia"}),
    );
    let discovery = KernelSpecManager::discover_in(&roots, &metadata);
    assert_eq!(
      discovery.recommended_id,
      discovery
        .kernels
        .iter()
        .find(|kernel| kernel.name == "python3")
        .map(|kernel| kernel.id.clone())
    );

    metadata.insert(
      "kernelspec".into(),
      serde_json::json!({"name": "missing", "language": "JULIA"}),
    );
    let discovery = KernelSpecManager::discover_in(&roots, &metadata);
    assert_eq!(
      discovery.recommended_id,
      discovery
        .kernels
        .iter()
        .find(|kernel| kernel.name == "julia")
        .map(|kernel| kernel.id.clone())
    );

    metadata.insert("kernelspec".into(), serde_json::json!({}));
    metadata.insert(
      "language_info".into(),
      serde_json::json!({"name": "python"}),
    );
    let discovery = KernelSpecManager::discover_in(&roots, &metadata);
    assert!(discovery.recommended_id.is_some());
  }

  #[test]
  fn no_kernels_is_a_successful_empty_discovery() {
    let directory = tempfile::tempdir().unwrap();
    let discovery = KernelSpecManager::discover_in(
      &[root(directory.path(), KernelSource::User)],
      &Metadata::new(),
    );

    assert_eq!(
      discovery,
      KernelDiscovery {
        diagnostics: Vec::new(),
        kernels: Vec::new(),
        recommended_id: None,
      }
    );
  }

  #[test]
  fn private_spec_retains_launch_data() {
    let directory = tempfile::tempdir().unwrap();
    let resource_dir = directory.path().join("foo");
    let kernel_file = resource_dir.join("kernel.json");
    write_spec(directory.path(), "foo", &valid_spec("Foo", "foo"));

    let spec = load_spec(
      "foo".into(),
      KernelSource::User,
      resource_dir.clone(),
      kernel_file.clone(),
    )
    .unwrap();

    assert_eq!(spec.argv, ["foo", "--connection", CONNECTION_FILE]);
    assert_eq!(spec.env["FOO"], "bar");
    assert_eq!(spec.resource_dir, resource_dir);
    assert_eq!(spec.kernel_file, kernel_file);
  }
}
