use {
  error::Error,
  kernel::LocalKernel,
  notebook::Notebook,
  serde::{Deserialize, Serialize, Serializer, de},
  serde_json::{Map, Value},
  std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
  },
  tempfile::Builder,
  thiserror::Error,
  typeshare::{U53, typeshare},
};

pub mod channel;
mod error;
pub mod kernel;
mod kernelspec;
mod notebook;
pub mod wire;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Default)]
struct KernelState(tokio::sync::Mutex<Option<LocalKernel>>);

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

#[tauri::command]
async fn select_kernel(
  name: Option<String>,
  state: tauri::State<'_, KernelState>,
) -> std::result::Result<(), String> {
  let mut current = state.0.lock().await;

  if let Some(kernel) = current.take() {
    kernel.shutdown().await.map_err(|error| error.to_string())?;
  }

  let Some(name) = name else {
    return Ok(());
  };

  let spec = tauri::async_runtime::spawn_blocking(move || {
    kernelspec::KernelSpecManager::launch_spec(&name)
  })
  .await
  .map_err(|error| error.to_string())??;
  let kernel = LocalKernel::launch(spec)
    .await
    .map_err(|error| error.to_string())?;

  current.replace(kernel);

  Ok(())
}

#[tauri::command]
async fn discover_kernelspecs(
  metadata: notebook::Metadata,
) -> Result<kernelspec::KernelDiscovery> {
  tauri::async_runtime::spawn_blocking(move || {
    kernelspec::KernelSpecManager::discover(&metadata)
  })
  .await
  .map_err(Error::Task)
}

/// # Panics
///
/// Panics if the Tauri application cannot run.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(KernelState::default())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      discover_kernelspecs,
      open_notebook,
      save_notebook,
      select_kernel
    ])
    .run(tauri::generate_context!())
    .expect("error while running Taipan");
}
