use super::*;

#[derive(Debug, Error)]
pub enum Error {
  #[error("failed to open {path}: {source}")]
  Open { path: PathBuf, source: io::Error },
  #[error("failed to parse {path}: {source}")]
  Parse {
    path: PathBuf,
    source: serde_json::Error,
  },
  #[error("unsupported notebook format {format} in {path}")]
  UnsupportedFormat { format: U53, path: PathBuf },
  #[error("cannot save unsupported notebook format {format}")]
  UnsupportedSaveFormat { format: U53 },
  #[error("failed to create temporary file in {path}: {source}")]
  CreateTemporary { path: PathBuf, source: io::Error },
  #[error("failed to preserve permissions for {path}: {source}")]
  PreservePermissions { path: PathBuf, source: io::Error },
  #[error("failed to serialize {path}: {source}")]
  Serialize {
    path: PathBuf,
    source: serde_json::Error,
  },
  #[error("failed to write {path}: {source}")]
  Write { path: PathBuf, source: io::Error },
  #[error("failed to flush {path}: {source}")]
  Flush { path: PathBuf, source: io::Error },
  #[error("failed to replace {path}: {source}")]
  Replace { path: PathBuf, source: io::Error },
  #[error("notebook task failed: {0}")]
  Task(#[source] tauri::Error),
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.collect_str(self)
  }
}
