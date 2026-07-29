use {
  chrono::{SecondsFormat, Utc},
  environment::Environment,
  error::Error,
  futures::{StreamExt, channel::mpsc as monitor},
  hmac::{Hmac, KeyInit, Mac},
  kernel_source::KernelSource,
  kernelspec::{KernelDiscovery, KernelSpecManager},
  notebook::Notebook,
  search_root::SearchRoot,
  serde::{Deserialize, Serialize, Serializer, de},
  serde_json::{Map, Value},
  sha2::Sha256,
  std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::{OsStr, OsString},
    fmt::{self, Display, Formatter},
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    net::{Ipv4Addr, TcpListener},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    str::FromStr,
    sync::{
      Arc,
      atomic::{AtomicBool, Ordering},
    },
    time::Duration,
  },
  tauri::{Emitter, Manager},
  tempfile::{Builder, NamedTempFile},
  thiserror::Error,
  tokio::{
    process::{Child, Command},
    sync::{mpsc, oneshot, watch},
    task::{JoinError, JoinHandle},
    time::{self, Instant},
  },
  typeshare::{U53, typeshare},
  uuid::Uuid,
  zeromq::{
    DealerRecvHalf, DealerSendHalf, DealerSocket, ReqSocket, Socket,
    SocketEvent, SocketOptions, SocketRecv, SocketSend, SubSocket, ZmqError,
    ZmqMessage, util::PeerIdentity,
  },
};

pub use {
  channel::{
    ChannelDriver, ChannelMessage, DriverConfig, HeartbeatDriver,
    TransportEvent,
  },
  kernel::{
    CellId, ConnectionData, DocumentId, ExecutionEvent, ExecutionId,
    ExecutionMessage, ExecutionRequest, ExecutionState, KernelChannels,
    KernelId, KernelInfo, KernelLaunchSpec, KernelState, LaunchConfig,
    LocalKernel, LocalKernelManager, ManagerConfig,
  },
  launch_error::LaunchError,
  manager_error::ManagerError,
  platform::Platform,
  transport_error::TransportError,
  wire::{
    Channel, DELIMITER, Envelope, Frame, Header, JsonFrame, JsonObject,
    MessageType, ParentHeader, WireProtocol,
  },
  wire_error::WireError,
};

#[cfg(unix)]
use nix::{
  errno::Errno,
  sys::signal::{Signal, killpg},
  unistd::Pid,
};

#[cfg(windows)]
use windows_sys::Win32::{
  Foundation::{CloseHandle, HANDLE},
  System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject,
    TerminateJobObject,
  },
};

#[cfg(test)]
use {
  std::process::Command as StdCommand,
  zeromq::{RepSocket, RouterSocket, XPubSocket},
};

mod channel;
mod environment;
mod error;
mod kernel;
mod kernel_source;
mod kernelspec;
mod launch_error;
mod manager_error;
mod notebook;
mod platform;
mod search_root;
mod transport_error;
mod wire;
mod wire_error;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Default)]
struct ApplicationState {
  current: Option<KernelId>,
  manager: LocalKernelManager,
}

#[derive(Clone, Copy, Serialize)]
struct KernelSelection {
  kernel_id: KernelId,
  state: KernelState,
}

#[derive(Clone, Copy, Serialize)]
struct KernelStatusEvent {
  kernel_id: KernelId,
  state: KernelState,
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
  app: tauri::AppHandle,
  state: tauri::State<'_, tokio::sync::Mutex<ApplicationState>>,
) -> std::result::Result<Option<KernelSelection>, String> {
  let mut state = state.inner().lock().await;

  if let Some(id) = state.current.take() {
    state
      .manager
      .shutdown(id)
      .await
      .map_err(|error| error.to_string())?;
  }

  let Some(name) = name else {
    return Ok(None);
  };

  let spec = tauri::async_runtime::spawn_blocking(move || {
    KernelSpecManager::launch_spec(&name)
  })
  .await
  .map_err(|error| error.to_string())??;
  let (events, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
  let id = state.manager.start_with_events(spec, events);
  let kernel_state = state
    .manager
    .wait_for_start(id)
    .await
    .map_err(|error| error.to_string())?;
  let mut status = state
    .manager
    .subscribe_state(id)
    .map_err(|error| error.to_string())?;

  state.current = Some(id);

  let event_app = app.clone();
  tauri::async_runtime::spawn(async move {
    while let Some(event) = event_receiver.recv().await {
      let _ = event_app.emit("execution-message", event);
    }
  });

  tauri::async_runtime::spawn(async move {
    while status.changed().await.is_ok() {
      let _ = app.emit(
        "kernel-status",
        KernelStatusEvent {
          kernel_id: id,
          state: *status.borrow_and_update(),
        },
      );
    }
  });

  Ok(Some(KernelSelection {
    kernel_id: id,
    state: kernel_state,
  }))
}

#[tauri::command]
async fn execute_cell(
  kernel_id: KernelId,
  document_id: DocumentId,
  cell_id: CellId,
  execution_id: ExecutionId,
  code: String,
  state: tauri::State<'_, tokio::sync::Mutex<ApplicationState>>,
) -> std::result::Result<(), String> {
  let state = state.inner().lock().await;

  if state.current != Some(kernel_id) {
    return Err("kernel is no longer selected".into());
  }

  state
    .manager
    .execute(
      kernel_id,
      ExecutionRequest {
        cell_id,
        code,
        document_id,
        execution_id,
      },
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn discover_kernelspecs() -> Result<KernelDiscovery> {
  tauri::async_runtime::spawn_blocking(kernelspec::KernelSpecManager::discover)
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
    .manage(tokio::sync::Mutex::new(ApplicationState::default()))
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      discover_kernelspecs,
      execute_cell,
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
          let state = app.state::<tokio::sync::Mutex<ApplicationState>>();
          state.lock().await.manager.shutdown_all().await;
        }

        app.exit(code.unwrap_or_default());
      });
    }
  });
}
