use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Source {
  Text(String),
  Lines(Vec<String>),
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Cell {
  pub cell_type: String,
  #[typeshare(typescript(type = "string | string[]"))]
  pub source: Source,
  #[typeshare(typescript(type = "Record<string, unknown>"))]
  pub metadata: Map<String, Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Notebook {
  pub cells: Vec<Cell>,
  #[typeshare(typescript(type = "Record<string, unknown>"))]
  pub metadata: Map<String, Value>,
  pub nbformat: U53,
  pub nbformat_minor: U53,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
}

impl Notebook {
  pub fn open(path: &Path) -> Result<Self> {
    let file = File::open(path).map_err(|source| Error::Open {
      path: path.into(),
      source,
    })?;

    let notebook = serde_json::from_reader::<_, Self>(BufReader::new(file))
      .map_err(|source| Error::Parse {
        path: path.into(),
        source,
      })?;

    if notebook.nbformat != 4 {
      return Err(Error::UnsupportedFormat {
        format: notebook.nbformat,
        path: path.into(),
      });
    }

    Ok(notebook)
  }

  pub fn save(&self, path: &Path) -> Result {
    if self.nbformat != 4 {
      return Err(Error::UnsupportedSaveFormat {
        format: self.nbformat,
      });
    }

    let parent = path
      .parent()
      .filter(|parent| !parent.as_os_str().is_empty())
      .unwrap_or_else(|| Path::new("."));

    let mut temporary = Builder::new()
      .prefix(".taipan-")
      .tempfile_in(parent)
      .map_err(|source| Error::CreateTemporary {
      path: parent.into(),
      source,
    })?;

    if let Ok(metadata) = fs::metadata(path) {
      temporary
        .as_file()
        .set_permissions(metadata.permissions())
        .map_err(|source| Error::PreservePermissions {
          path: path.into(),
          source,
        })?;
    }

    {
      let mut writer = BufWriter::new(temporary.as_file_mut());

      serde_json::to_writer_pretty(&mut writer, self).map_err(|source| {
        Error::Serialize {
          path: path.into(),
          source,
        }
      })?;

      writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|source| Error::Write {
          path: path.into(),
          source,
        })?;
    }

    temporary
      .as_file()
      .sync_all()
      .map_err(|source| Error::Flush {
        path: path.into(),
        source,
      })?;

    temporary.persist(path).map_err(|error| Error::Replace {
      path: path.into(),
      source: error.error,
    })?;

    #[cfg(unix)]
    File::open(parent)
      .and_then(|directory| directory.sync_all())
      .map_err(|source| Error::Flush {
        path: parent.into(),
        source,
      })?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const FIXTURE: &str = include_str!("../tests/fixtures/round-trip.ipynb");

  #[test]
  fn round_trip_preserves_notebook() {
    let notebook = serde_json::from_str::<Notebook>(FIXTURE).unwrap();

    assert_eq!(
      serde_json::to_value(notebook).unwrap(),
      serde_json::from_str::<Value>(FIXTURE).unwrap()
    );
  }

  #[test]
  fn save_replaces_only_edited_source() {
    let directory = tempfile::tempdir().unwrap();

    let path = directory.path().join("foo.ipynb");
    fs::write(&path, FIXTURE).unwrap();

    let mut notebook = Notebook::open(&path).unwrap();
    notebook.cells[0].source = Source::Text("bar".into());

    notebook.save(&path).unwrap();

    let actual = serde_json::to_value(Notebook::open(&path).unwrap()).unwrap();

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

    assert!(matches!(
      Notebook::open(&path),
      Err(Error::UnsupportedFormat { .. })
    ));
  }
}
