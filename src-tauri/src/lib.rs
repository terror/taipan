use {
  error::Error,
  kernel::{KernelId, LocalKernelManager},
  notebook::Notebook,
  serde::{Deserialize, Serialize, Serializer, de},
  serde_json::{Map, Value},
  std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
      Arc,
      atomic::{AtomicBool, Ordering},
    },
  },
  tauri::Manager,
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
struct KernelState {
  current: Option<KernelId>,
  manager: LocalKernelManager,
}

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
  state: tauri::State<'_, tokio::sync::Mutex<KernelState>>,
) -> std::result::Result<(), String> {
  let mut state = state.inner().lock().await;

  if let Some(id) = state.current.take() {
    state
      .manager
      .shutdown(id)
      .await
      .map_err(|error| error.to_string())?;
  }

  let Some(name) = name else {
    return Ok(());
  };

  let spec = tauri::async_runtime::spawn_blocking(move || {
    kernelspec::KernelSpecManager::launch_spec(&name)
  })
  .await
  .map_err(|error| error.to_string())??;
  let id = state.manager.start(spec);
  state
    .manager
    .wait_for_start(id)
    .await
    .map_err(|error| error.to_string())?;

  state.current = Some(id);

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
  let exiting = Arc::new(AtomicBool::new(false));
  let app = tauri::Builder::default()
    .manage(tokio::sync::Mutex::new(KernelState::default()))
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      discover_kernelspecs,
      open_notebook,
      save_notebook,
      select_kernel
    ])
    .build(tauri::generate_context!())
    .expect("error while building Taipan");

  app.run(move |app, event| {
    if let tauri::RunEvent::ExitRequested { api, code, .. } = event
      && !exiting.swap(true, Ordering::Relaxed)
    {
      api.prevent_exit();

      let app = app.clone();
      tauri::async_runtime::spawn(async move {
        {
          let state = app.state::<tokio::sync::Mutex<KernelState>>();
          state.lock().await.manager.shutdown_all().await;
        }

        app.exit(code.unwrap_or_default());
      });
    }
  });
}
