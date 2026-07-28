use super::*;

#[typeshare(serialized_as = "HashMap<String, MimeBundle>")]
pub type Attachments = BTreeMap<String, MimeBundle>;

#[typeshare(serialized_as = "HashMap<String, Value>")]
pub type Metadata = Map<String, Value>;

#[typeshare(serialized_as = "HashMap<String, Value>")]
pub type MimeBundle = Map<String, Value>;

#[typeshare(serialized_as = "MultilineString")]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Source {
  Lines(Vec<String>),
  Text(String),
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CodeCell {
  #[typeshare(typescript(type = "\"code\""))]
  pub cell_type: String,
  pub execution_count: ExecutionCount,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub metadata: Metadata,
  pub outputs: Vec<NotebookOutput>,
  pub source: Source,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DisplayDataOutput {
  pub data: MimeBundle,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  pub metadata: Metadata,
  #[typeshare(typescript(type = "\"display_data\""))]
  pub output_type: String,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ErrorOutput {
  pub ename: String,
  pub evalue: String,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  #[typeshare(typescript(type = "\"error\""))]
  pub output_type: String,
  pub traceback: Vec<String>,
}

#[typeshare(serialized_as = "NullableExecutionCount")]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ExecutionCount(pub Option<U53>);

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExecuteResultOutput {
  pub data: MimeBundle,
  pub execution_count: ExecutionCount,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  pub metadata: Metadata,
  #[typeshare(typescript(type = "\"execute_result\""))]
  pub output_type: String,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MarkdownCell {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub attachments: Option<Attachments>,
  #[typeshare(typescript(type = "\"markdown\""))]
  pub cell_type: String,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub metadata: Metadata,
  pub source: Source,
}

#[typeshare(serialized_as = "NotebookCellType")]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum NotebookCell {
  Code(CodeCell),
  Markdown(MarkdownCell),
  Raw(RawCell),
  Unknown(UnknownCell),
}

impl<'de> Deserialize<'de> for NotebookCell {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let value = Value::deserialize(deserializer)?;

    let cell_type = value
      .get("cell_type")
      .and_then(Value::as_str)
      .ok_or_else(|| de::Error::custom("cell_type must be a string"))?
      .to_owned();

    match cell_type.as_str() {
      "code" => serde_json::from_value(value).map(Self::Code),
      "markdown" => serde_json::from_value(value).map(Self::Markdown),
      "raw" => serde_json::from_value(value).map(Self::Raw),
      _ => serde_json::from_value(value).map(Self::Unknown),
    }
    .map_err(de::Error::custom)
  }
}

#[typeshare(serialized_as = "NotebookOutputType")]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum NotebookOutput {
  DisplayData(DisplayDataOutput),
  Error(ErrorOutput),
  ExecuteResult(ExecuteResultOutput),
  Stream(StreamOutput),
}

impl<'de> Deserialize<'de> for NotebookOutput {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let value = Value::deserialize(deserializer)?;

    let output_type = value
      .get("output_type")
      .and_then(Value::as_str)
      .ok_or_else(|| de::Error::custom("output_type must be a string"))?
      .to_owned();

    match output_type.as_str() {
      "display_data" => serde_json::from_value(value).map(Self::DisplayData),
      "error" => serde_json::from_value(value).map(Self::Error),
      "execute_result" => {
        serde_json::from_value(value).map(Self::ExecuteResult)
      }
      "stream" => serde_json::from_value(value).map(Self::Stream),
      _ => {
        return Err(de::Error::custom(format!(
          "unsupported notebook output type `{output_type}`"
        )));
      }
    }
    .map_err(de::Error::custom)
  }
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RawCell {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub attachments: Option<Attachments>,
  #[typeshare(typescript(type = "\"raw\""))]
  pub cell_type: String,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub metadata: Metadata,
  pub source: Source,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StreamOutput {
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  pub name: String,
  #[typeshare(typescript(type = "\"stream\""))]
  pub output_type: String,
  pub text: Source,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UnknownCell {
  pub cell_type: String,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub metadata: Metadata,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub source: Option<Source>,
}

#[typeshare]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Notebook {
  pub cells: Vec<NotebookCell>,
  #[typeshare(skip)]
  #[serde(flatten)]
  pub extra: Map<String, Value>,
  pub metadata: Metadata,
  pub nbformat: U53,
  pub nbformat_minor: U53,
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
  fn parses_all_cell_and_output_variants() {
    let notebook = serde_json::from_str::<Notebook>(FIXTURE).unwrap();

    assert!(matches!(notebook.cells[1], NotebookCell::Markdown(_)));
    assert!(matches!(notebook.cells[2], NotebookCell::Raw(_)));
    assert!(matches!(notebook.cells[3], NotebookCell::Unknown(_)));

    let NotebookCell::Code(code) = &notebook.cells[0] else {
      panic!();
    };

    assert!(matches!(
      code.outputs.as_slice(),
      [
        NotebookOutput::Stream(_),
        NotebookOutput::Error(_),
        NotebookOutput::ExecuteResult(_),
        NotebookOutput::DisplayData(_),
      ]
    ));

    let NotebookOutput::ExecuteResult(output) = &code.outputs[2] else {
      panic!();
    };

    assert_eq!(output.data["application/x-unsupported"], 42);

    let NotebookCell::Markdown(markdown) = &notebook.cells[1] else {
      panic!();
    };

    assert_eq!(
      markdown.attachments.as_ref().unwrap()["foo.png"]["application/x-unsupported"],
      false
    );
  }

  #[test]
  fn rejects_transient_output() {
    assert!(
      serde_json::from_value::<NotebookOutput>(serde_json::json!({
        "data": {},
        "metadata": {},
        "output_type": "update_display_data"
      }))
      .is_err()
    );
  }

  #[test]
  fn round_trip_preserves_all_notebook_data() {
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
    let NotebookCell::Code(cell) = &mut notebook.cells[0] else {
      panic!();
    };

    cell.source = Source::Text("bar".into());

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
