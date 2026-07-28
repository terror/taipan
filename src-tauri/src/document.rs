use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
  fs::{self, File},
  io::{BufReader, BufWriter, Write},
  path::Path,
};
use tempfile::Builder;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Source {
  Text(String),
  Lines(Vec<String>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Cell {
  pub cell_type: String,
  pub source: Source,
  pub metadata: Map<String, Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(flatten)]
  pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NotebookDocument {
  pub cells: Vec<Cell>,
  pub metadata: Map<String, Value>,
  pub nbformat: u64,
  pub nbformat_minor: u64,
  #[serde(flatten)]
  pub extra: Map<String, Value>,
}

pub fn open(path: &Path) -> Result<NotebookDocument, String> {
  let file = File::open(path)
    .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
  let notebook =
    serde_json::from_reader::<_, NotebookDocument>(BufReader::new(file))
      .map_err(|error| {
        format!("failed to parse {}: {error}", path.display())
      })?;

  if notebook.nbformat != 4 {
    return Err(format!(
      "unsupported notebook format {} in {}",
      notebook.nbformat,
      path.display()
    ));
  }

  Ok(notebook)
}

pub fn save(path: &Path, notebook: &NotebookDocument) -> Result<(), String> {
  if notebook.nbformat != 4 {
    return Err(format!(
      "cannot save unsupported notebook format {}",
      notebook.nbformat
    ));
  }

  let parent = path
    .parent()
    .filter(|parent| !parent.as_os_str().is_empty())
    .unwrap_or_else(|| Path::new("."));

  let mut temporary = Builder::new()
    .prefix(".taipan-")
    .tempfile_in(parent)
    .map_err(|error| {
      format!(
        "failed to create temporary file in {}: {error}",
        parent.display()
      )
    })?;

  if let Ok(metadata) = fs::metadata(path) {
    temporary
      .as_file()
      .set_permissions(metadata.permissions())
      .map_err(|error| {
        format!(
          "failed to preserve permissions for {}: {error}",
          path.display()
        )
      })?;
  }

  {
    let mut writer = BufWriter::new(temporary.as_file_mut());
    serde_json::to_writer_pretty(&mut writer, notebook).map_err(|error| {
      format!("failed to serialize {}: {error}", path.display())
    })?;
    writer
      .write_all(b"\n")
      .and_then(|()| writer.flush())
      .map_err(|error| {
        format!("failed to write {}: {error}", path.display())
      })?;
  }

  temporary
    .as_file()
    .sync_all()
    .map_err(|error| format!("failed to flush {}: {error}", path.display()))?;

  temporary.persist(path).map_err(|error| {
    format!("failed to replace {}: {}", path.display(), error.error)
  })?;

  #[cfg(unix)]
  File::open(parent)
    .and_then(|directory| directory.sync_all())
    .map_err(|error| {
      format!("failed to flush {}: {error}", parent.display())
    })?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  const FIXTURE: &str = include_str!("../tests/fixtures/round-trip.ipynb");

  #[test]
  fn round_trip_preserves_notebook() {
    let notebook = serde_json::from_str::<NotebookDocument>(FIXTURE).unwrap();
    let actual = serde_json::to_value(notebook).unwrap();
    let expected = serde_json::from_str::<Value>(FIXTURE).unwrap();

    assert_eq!(actual, expected);
  }

  #[test]
  fn save_replaces_only_edited_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foo.ipynb");
    fs::write(&path, FIXTURE).unwrap();

    let mut notebook = open(&path).unwrap();
    notebook.cells[0].source = Source::Text("bar".into());
    save(&path, &notebook).unwrap();

    let actual = serde_json::to_value(open(&path).unwrap()).unwrap();
    let mut expected = serde_json::from_str::<Value>(FIXTURE).unwrap();
    expected["cells"][0]["source"] = Value::String("bar".into());

    assert_eq!(actual, expected);
  }

  #[test]
  fn rejects_unsupported_format() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foo.ipynb");
    fs::write(&path, FIXTURE.replace("\"nbformat\": 4", "\"nbformat\": 3"))
      .unwrap();

    assert!(
      open(&path)
        .unwrap_err()
        .contains("unsupported notebook format 3")
    );
  }
}
