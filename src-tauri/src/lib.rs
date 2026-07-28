use {
  error::Error,
  notebook::Notebook,
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

mod error;
mod notebook;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tauri::command]
async fn open_notebook(path: PathBuf) -> Result<Notebook> {
  tauri::async_runtime::spawn_blocking(move || Notebook::open(&path))
    .await
    .map_err(Error::Task)?
}

#[tauri::command]
async fn save_notebook(path: PathBuf, notebook: Notebook) -> Result {
  tauri::async_runtime::spawn_blocking(move || notebook.save(&path))
    .await
    .map_err(Error::Task)?
}

/// # Panics
///
/// Panics if the Tauri application cannot run.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![open_notebook, save_notebook])
    .run(tauri::generate_context!())
    .expect("error while running Taipan");
}
