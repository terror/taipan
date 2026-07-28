use {
  document::NotebookDocument,
  error::Error,
  serde::{Deserialize, Serialize, Serializer},
  serde_json::{Map, Value},
  std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
  },
  tempfile::Builder,
  thiserror::Error,
  typeshare::{U53, typeshare},
};

mod document;
mod error;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tauri::command]
async fn open_notebook(path: PathBuf) -> Result<NotebookDocument> {
  tauri::async_runtime::spawn_blocking(move || document::open(&path))
    .await
    .map_err(Error::Task)?
}

#[tauri::command]
async fn save_notebook(path: PathBuf, notebook: NotebookDocument) -> Result {
  tauri::async_runtime::spawn_blocking(move || document::save(&path, &notebook))
    .await
    .map_err(Error::Task)?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![open_notebook, save_notebook])
    .run(tauri::generate_context!())
    .expect("error while running Taipan");
}
