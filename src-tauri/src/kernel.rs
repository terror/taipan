use {
  crate::{
    channel::{
      ChannelDriver, DriverConfig, HeartbeatDriver, TransportError,
      TransportEvent,
    },
    wire::{
      Channel, Envelope, Header, JsonObject, MessageType, ParentHeader,
      WireError, WireProtocol,
    },
  },
  chrono::{SecondsFormat, Utc},
  serde::Serialize,
  serde_json::Value,
  std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::{OsStr, OsString},
    fmt,
    io::{self, Write},
    net::{Ipv4Addr, TcpListener},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
  },
  tempfile::{Builder, NamedTempFile},
  thiserror::Error,
  tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    sync::mpsc,
    task::JoinHandle,
    time::{self, Instant},
  },
  uuid::Uuid,
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

const CONNECTION_FILE: &str = "{connection_file}";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConnectionData {
  pub control_port: u16,
  pub hb_port: u16,
  pub iopub_port: u16,
  pub ip: String,
  pub key: String,
  pub shell_port: u16,
  pub signature_scheme: String,
  pub stdin_port: u16,
  pub transport: String,
}

impl ConnectionData {
  fn allocate() -> Result<(Self, Vec<TcpListener>), LaunchError> {
    let listeners = (0..5)
      .map(|_| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)))
      .collect::<Result<Vec<_>, _>>()
      .map_err(LaunchError::AllocatePorts)?;
    let ports = listeners
      .iter()
      .map(|listener| listener.local_addr().map(|address| address.port()))
      .collect::<Result<Vec<_>, _>>()
      .map_err(LaunchError::AllocatePorts)?;

    Ok((
      Self {
        control_port: ports[0],
        hb_port: ports[1],
        ip: Ipv4Addr::LOCALHOST.to_string(),
        iopub_port: ports[2],
        key: Uuid::new_v4().to_string(),
        shell_port: ports[3],
        signature_scheme: "hmac-sha256".into(),
        stdin_port: ports[4],
        transport: "tcp".into(),
      },
      listeners,
    ))
  }

  #[must_use]
  pub fn endpoint(&self, channel: Channel) -> String {
    let port = match channel {
      Channel::Control => self.control_port,
      Channel::Heartbeat => self.hb_port,
      Channel::Iopub => self.iopub_port,
      Channel::Shell => self.shell_port,
      Channel::Stdin => self.stdin_port,
    };

    format!("{}://{}:{port}", self.transport, self.ip)
  }
}

#[derive(Clone, Debug)]
pub struct KernelLaunchSpec {
  pub argv: Vec<String>,
  pub env: BTreeMap<String, String>,
  pub language: String,
  pub resource_dir: Option<PathBuf>,
}

impl KernelLaunchSpec {
  #[must_use]
  pub fn new(
    argv: Vec<String>,
    env: BTreeMap<String, String>,
    language: impl Into<String>,
  ) -> Self {
    Self {
      argv,
      env,
      language: language.into(),
      resource_dir: None,
    }
  }
}

#[derive(Clone, Debug)]
pub struct LaunchConfig {
  pub max_startup_output_bytes: usize,
  pub runtime_dir: Option<PathBuf>,
  pub startup_timeout: Duration,
}

impl Default for LaunchConfig {
  fn default() -> Self {
    Self {
      max_startup_output_bytes: 16 * 1024,
      runtime_dir: None,
      startup_timeout: Duration::from_secs(15),
    }
  }
}

#[derive(Debug, Error)]
pub enum LaunchError {
  #[error("failed to allocate local kernel ports")]
  AllocatePorts(#[source] io::Error),
  #[error("failed to create private kernel connection file")]
  ConnectionFile(#[source] io::Error),
  #[error("failed to serialize kernel connection data")]
  ConnectionJson(#[source] serde_json::Error),
  #[error("invalid kernel command")]
  InvalidCommand,
  #[error("invalid environment template for `{0}`")]
  InvalidEnvironmentTemplate(String),
  #[error("failed to prepare Jupyter wire protocol")]
  Protocol(#[source] WireError),
  #[error("failed to spawn kernel process")]
  Spawn(#[source] io::Error),
  #[error("kernel startup failed: {reason}{output}")]
  Startup {
    output: StartupOutput,
    reason: String,
  },
  #[error("failed to stop kernel process")]
  Stop(#[source] io::Error),
  #[error("failed to connect kernel channel")]
  Transport(#[source] TransportError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelInfo {
  pub banner: String,
  pub implementation: String,
  pub implementation_version: String,
  pub language_info: JsonObject,
  pub protocol_version: String,
}

pub struct KernelChannels {
  pub control: ChannelDriver,
  pub control_events: mpsc::Receiver<TransportEvent>,
  pub heartbeat: HeartbeatDriver,
  pub heartbeat_events: mpsc::Receiver<TransportEvent>,
  pub iopub: ChannelDriver,
  pub iopub_events: mpsc::Receiver<TransportEvent>,
  pub shell: ChannelDriver,
  pub shell_events: mpsc::Receiver<TransportEvent>,
  pub stdin: ChannelDriver,
  pub stdin_events: mpsc::Receiver<TransportEvent>,
}

impl KernelChannels {
  async fn connect(
    connection: &ConnectionData,
    protocol: Arc<WireProtocol>,
    config: DriverConfig,
  ) -> Result<Self, TransportError> {
    let (iopub, iopub_events) = ChannelDriver::connect(
      Channel::Iopub,
      &connection.endpoint(Channel::Iopub),
      protocol.clone(),
      config.clone(),
    )
    .await?;
    let (shell, shell_events) = ChannelDriver::connect(
      Channel::Shell,
      &connection.endpoint(Channel::Shell),
      protocol.clone(),
      config.clone(),
    )
    .await?;
    let (control, control_events) = ChannelDriver::connect(
      Channel::Control,
      &connection.endpoint(Channel::Control),
      protocol.clone(),
      config.clone(),
    )
    .await?;
    let (stdin, stdin_events) = ChannelDriver::connect(
      Channel::Stdin,
      &connection.endpoint(Channel::Stdin),
      protocol,
      config.clone(),
    )
    .await?;
    let (heartbeat, heartbeat_events) = HeartbeatDriver::connect(
      &connection.endpoint(Channel::Heartbeat),
      config,
    )
    .await?;

    Ok(Self {
      control,
      control_events,
      heartbeat,
      heartbeat_events,
      iopub,
      iopub_events,
      shell,
      shell_events,
      stdin,
      stdin_events,
    })
  }

  async fn shutdown(self) {
    let Self {
      control,
      control_events: _,
      heartbeat,
      heartbeat_events: _,
      iopub,
      iopub_events: _,
      shell,
      shell_events: _,
      stdin,
      stdin_events: _,
    } = self;

    let _ = control.shutdown().await;
    let _ = heartbeat.shutdown().await;
    let _ = iopub.shutdown().await;
    let _ = shell.shutdown().await;
    let _ = stdin.shutdown().await;
  }
}

pub struct LocalKernel {
  pub channels: Option<KernelChannels>,
  info: KernelInfo,
  process: Option<KernelProcess>,
}

impl LocalKernel {
  #[must_use]
  pub fn info(&self) -> &KernelInfo {
    &self.info
  }

  #[allow(clippy::missing_errors_doc)]
  pub async fn launch(spec: KernelLaunchSpec) -> Result<Self, LaunchError> {
    Self::launch_with_config(spec, LaunchConfig::default()).await
  }

  #[allow(clippy::missing_errors_doc)]
  pub async fn launch_with_config(
    spec: KernelLaunchSpec,
    config: LaunchConfig,
  ) -> Result<Self, LaunchError> {
    if spec.argv.first().is_none_or(String::is_empty) {
      return Err(LaunchError::InvalidCommand);
    }

    let (connection, reservations) = ConnectionData::allocate()?;
    let connection_file = write_connection_file(&connection, &config)?;
    let argv = substitute_argv(&spec.argv, connection_file.path());
    let inherited = env::vars_os().collect::<BTreeMap<_, _>>();
    let base = sanitized_environment(&inherited);
    let mut environment = expand_environment(&base, &spec.env)?;

    if spec.language.to_ascii_lowercase().starts_with("python") {
      environment.remove(OsStr::new("PYTHONEXECUTABLE"));
    }
    let redactor = Redactor::new(
      &spec,
      &inherited,
      &environment,
      &connection,
      connection_file.path(),
    );
    let capture = Arc::new(Mutex::new(StartupCapture::new(
      config.max_startup_output_bytes,
      redactor.maximum_value_length(),
    )));
    let protocol = Arc::new(
      WireProtocol::new(
        connection.key.as_bytes(),
        &connection.signature_scheme,
      )
      .map_err(LaunchError::Protocol)?,
    );
    let session = Uuid::new_v4().to_string();
    let driver_config = DriverConfig {
      client_identity: session.as_bytes().to_vec(),
      ..DriverConfig::default()
    };
    drop(reservations);

    let mut process = KernelProcess::spawn(
      &argv,
      &environment,
      connection_file,
      capture,
      redactor,
    )?;
    let deadline = Instant::now() + config.startup_timeout;
    let channels = time::timeout_at(
      deadline,
      KernelChannels::connect(&connection, protocol, driver_config),
    )
    .await;
    let mut channels = match channels {
      Ok(Ok(channels)) => channels,
      Ok(Err(error)) => {
        let cleanup = process.stop().await.err();
        let output = process.startup_output();
        return Err(LaunchError::Startup {
          output,
          reason: startup_reason(error.to_string(), cleanup),
        });
      }
      Err(_) => {
        let cleanup = process.stop().await.err();
        let output = process.startup_output();
        return Err(LaunchError::Startup {
          output,
          reason: startup_reason(
            format!("timed out after {:?}", config.startup_timeout),
            cleanup,
          ),
        });
      }
    };
    let readiness = time::timeout_at(
      deadline,
      establish_readiness(&mut process, &mut channels, &session),
    )
    .await;
    let info = match readiness {
      Ok(Ok(info)) => info,
      Ok(Err(reason)) => {
        channels.shutdown().await;
        let cleanup = process.stop().await.err();
        let output = process.startup_output();
        return Err(LaunchError::Startup {
          output,
          reason: startup_reason(reason, cleanup),
        });
      }
      Err(_) => {
        channels.shutdown().await;
        let cleanup = process.stop().await.err();
        let output = process.startup_output();
        return Err(LaunchError::Startup {
          output,
          reason: startup_reason(
            format!("timed out after {:?}", config.startup_timeout),
            cleanup,
          ),
        });
      }
    };

    Ok(Self {
      channels: Some(channels),
      info,
      process: Some(process),
    })
  }

  #[allow(clippy::missing_errors_doc)]
  pub async fn shutdown(mut self) -> Result<(), LaunchError> {
    if let Some(channels) = self.channels.take() {
      channels.shutdown().await;
    }

    if let Some(mut process) = self.process.take() {
      process.stop().await.map_err(LaunchError::Stop)?;
    }

    Ok(())
  }
}

impl Drop for LocalKernel {
  fn drop(&mut self) {
    if let Some(channels) = &self.channels {
      channels.control.cancel();
      channels.heartbeat.cancel();
      channels.iopub.cancel();
      channels.shell.cancel();
      channels.stdin.cancel();
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupOutput {
  stderr: String,
  stdout: String,
  truncated: bool,
}

impl fmt::Display for StartupOutput {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.stdout.is_empty() && self.stderr.is_empty() {
      return Ok(());
    }

    formatter.write_str("\nstartup output:")?;

    if !self.stdout.is_empty() {
      write!(formatter, "\nstdout: {}", self.stdout.trim_end())?;
    }

    if !self.stderr.is_empty() {
      write!(formatter, "\nstderr: {}", self.stderr.trim_end())?;
    }

    if self.truncated {
      formatter.write_str("\n<output truncated>")?;
    }

    Ok(())
  }
}

struct KernelProcess {
  capture: Arc<Mutex<StartupCapture>>,
  child: Child,
  connection_file: NamedTempFile,
  #[cfg(unix)]
  process_group: Option<u32>,
  readers: Vec<JoinHandle<io::Result<()>>>,
  redactor: Redactor,
  #[cfg(windows)]
  windows_job: WindowsJob,
}

impl KernelProcess {
  fn spawn(
    argv: &[String],
    environment: &BTreeMap<OsString, OsString>,
    connection_file: NamedTempFile,
    capture: Arc<Mutex<StartupCapture>>,
    redactor: Redactor,
  ) -> Result<Self, LaunchError> {
    let mut command = Command::new(&argv[0]);
    command
      .args(&argv[1..])
      .env_clear()
      .envs(environment)
      .kill_on_drop(true)
      .stdin(Stdio::null())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());

    #[cfg(unix)]
    command.process_group(0);

    let mut child = command.spawn().map_err(LaunchError::Spawn)?;
    #[cfg(unix)]
    let process_group = child.id();
    #[cfg(windows)]
    let windows_job = WindowsJob::new(&child).map_err(LaunchError::Spawn)?;
    let stdout = child.stdout.take().ok_or_else(|| {
      LaunchError::Spawn(io::Error::other("stdout pipe unavailable"))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
      LaunchError::Spawn(io::Error::other("stderr pipe unavailable"))
    })?;
    let readers = vec![
      tokio::spawn(drain_output(stdout, capture.clone(), OutputStream::Stdout)),
      tokio::spawn(drain_output(stderr, capture.clone(), OutputStream::Stderr)),
    ];

    Ok(Self {
      capture,
      child,
      connection_file,
      #[cfg(unix)]
      process_group,
      readers,
      redactor,
      #[cfg(windows)]
      windows_job,
    })
  }

  fn start_kill(&mut self) -> io::Result<()> {
    #[cfg(unix)]
    {
      let Some(process_group) = self.process_group else {
        return Ok(());
      };

      match killpg(Pid::from_raw(process_group.cast_signed()), Signal::SIGKILL)
      {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(error) => Err(io::Error::from_raw_os_error(error as i32)),
      }
    }

    #[cfg(not(unix))]
    {
      #[cfg(windows)]
      return self.windows_job.terminate();

      #[cfg(not(windows))]
      self.child.start_kill()
    }
  }

  fn startup_output(&self) -> StartupOutput {
    self.capture.lock().map_or_else(
      |_| StartupOutput {
        stderr: String::new(),
        stdout: String::new(),
        truncated: true,
      },
      |capture| capture.output(&self.redactor),
    )
  }

  async fn stop(&mut self) -> io::Result<()> {
    let kill = self.start_kill();
    let wait = time::timeout(Duration::from_secs(3), self.child.wait()).await;

    for mut reader in self.readers.drain(..) {
      if time::timeout(Duration::from_millis(100), &mut reader)
        .await
        .is_err()
      {
        reader.abort();
        let _ = reader.await;
      }
    }

    kill?;

    match wait {
      Ok(Ok(_)) => Ok(()),
      Ok(Err(error)) => Err(error),
      Err(_) => Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "kernel did not exit after termination",
      )),
    }
  }
}

impl Drop for KernelProcess {
  fn drop(&mut self) {
    let _ = self.start_kill();

    for reader in &self.readers {
      reader.abort();
    }

    let _ = self.connection_file.as_file();
  }
}

#[derive(Clone, Copy)]
enum OutputStream {
  Stderr,
  Stdout,
}

struct Redactor {
  values: Vec<String>,
}

impl Redactor {
  fn maximum_value_length(&self) -> usize {
    self.values.first().map_or(0, String::len)
  }

  fn new(
    spec: &KernelLaunchSpec,
    inherited: &BTreeMap<OsString, OsString>,
    environment: &BTreeMap<OsString, OsString>,
    connection: &ConnectionData,
    connection_file: &Path,
  ) -> Self {
    let mut values = vec![
      connection.key.clone(),
      connection_file.to_string_lossy().into_owned(),
    ];

    if let Some(resource_dir) = &spec.resource_dir {
      values.push(resource_dir.to_string_lossy().into_owned());
    }

    if let Ok(directory) = env::current_dir() {
      values.push(directory.to_string_lossy().into_owned());
    }

    for name in ["HOME", "USERPROFILE"] {
      if let Some(value) = inherited.get(OsStr::new(name)) {
        values.push(value.to_string_lossy().into_owned());
      }
    }

    for (name, value) in inherited.iter().chain(environment) {
      if secret_name(name) {
        values.push(value.to_string_lossy().into_owned());
      }
    }

    values.retain(|value| value.len() >= 4);
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values.dedup();

    Self { values }
  }

  fn sanitize(&self, bytes: &[u8]) -> String {
    let mut value = strip_controls(&String::from_utf8_lossy(bytes));

    for sensitive in &self.values {
      let replacement = if sensitive.len() <= "<redacted>".len() {
        "<redacted>".into()
      } else {
        format!(
          "<redacted>{}",
          "*".repeat(sensitive.len() - "<redacted>".len())
        )
      };

      value = value.replace(sensitive, &replacement);
    }

    value
  }
}

struct StartupCapture {
  maximum: usize,
  retention: usize,
  stderr: Vec<u8>,
  stdout: Vec<u8>,
  truncated: bool,
}

impl StartupCapture {
  fn append(&mut self, stream: OutputStream, bytes: &[u8]) {
    let target = match stream {
      OutputStream::Stderr => &mut self.stderr,
      OutputStream::Stdout => &mut self.stdout,
    };
    let available = self.retention.saturating_sub(target.len());
    let retained = bytes.len().min(available);

    target.extend_from_slice(&bytes[..retained]);
    self.truncated |= retained < bytes.len();
  }

  fn new(maximum: usize, redaction_overlap: usize) -> Self {
    Self {
      maximum,
      retention: maximum.saturating_add(redaction_overlap),
      stderr: Vec::new(),
      stdout: Vec::new(),
      truncated: false,
    }
  }

  fn output(&self, redactor: &Redactor) -> StartupOutput {
    let mut stdout = redactor.sanitize(&self.stdout);
    let mut stderr = redactor.sanitize(&self.stderr);
    let stdout_truncated = truncate_utf8(&mut stdout, self.maximum);
    let stderr_truncated =
      truncate_utf8(&mut stderr, self.maximum.saturating_sub(stdout.len()));

    StartupOutput {
      stderr,
      stdout,
      truncated: self.truncated || stderr_truncated || stdout_truncated,
    }
  }
}

async fn drain_output(
  mut reader: impl AsyncRead + Unpin,
  capture: Arc<Mutex<StartupCapture>>,
  stream: OutputStream,
) -> io::Result<()> {
  let mut bytes = [0_u8; 4_096];

  loop {
    let count = reader.read(&mut bytes).await?;

    if count == 0 {
      return Ok(());
    }

    if let Ok(mut capture) = capture.lock() {
      capture.append(stream, &bytes[..count]);
    }
  }
}

async fn establish_readiness(
  process: &mut KernelProcess,
  channels: &mut KernelChannels,
  session: &str,
) -> Result<KernelInfo, String> {
  let ping = Uuid::new_v4().as_bytes().to_vec();
  channels
    .heartbeat
    .try_ping(ping.clone())
    .map_err(|error| error.to_string())?;
  let mut heartbeat_ready = false;
  let mut info = None;
  let mut iopub_ready = false;
  let mut requests = BTreeSet::new();
  let mut request_interval = time::interval_at(
    Instant::now() + Duration::from_millis(250),
    Duration::from_millis(250),
  );
  let mut process_interval = time::interval(Duration::from_millis(25));

  send_kernel_info(&channels.shell, session, &mut requests)
    .map_err(|error| error.to_string())?;

  loop {
    if heartbeat_ready
      && iopub_ready
      && let Some(info) = info.take()
    {
      return Ok(info);
    }

    tokio::select! {
      event = channels.heartbeat_events.recv() => match event {
        Some(TransportEvent::Heartbeat(bytes)) if bytes == ping => {
          heartbeat_ready = true;
        }
        Some(TransportEvent::Heartbeat(_)) => {
          return Err("heartbeat reply did not match probe".into());
        }
        Some(TransportEvent::Error { error, .. }) => {
          return Err(error.to_string());
        }
        Some(TransportEvent::Message(_)) => {
          return Err("heartbeat channel received a Jupyter message".into());
        }
        None => return Err("heartbeat channel closed during startup".into()),
      },
      event = channels.shell_events.recv() => match event {
        Some(TransportEvent::Message(message)) => {
          let envelope = &message.envelope;

          if envelope.header.msg_type == MessageType::from("kernel_info_reply")
            && correlated(envelope, &requests)
          {
            info = Some(validate_kernel_info(envelope)?);
          }
        }
        Some(TransportEvent::Error { error, .. }) => {
          return Err(error.to_string());
        }
        Some(TransportEvent::Heartbeat(_)) => {
          return Err("shell channel received a heartbeat".into());
        }
        None => return Err("shell channel closed during startup".into()),
      },
      event = channels.iopub_events.recv() => match event {
        Some(TransportEvent::Message(message)) => {
          let envelope = &message.envelope;

          if valid_iopub_welcome(envelope)
            || valid_correlated_status(envelope, &requests)
          {
            iopub_ready = true;
          }
        }
        Some(TransportEvent::Error { error, .. }) => {
          return Err(error.to_string());
        }
        Some(TransportEvent::Heartbeat(_)) => {
          return Err("IOPub channel received a heartbeat".into());
        }
        None => return Err("IOPub channel closed during startup".into()),
      },
      _ = request_interval.tick() => {
        send_kernel_info(&channels.shell, session, &mut requests)
          .map_err(|error| error.to_string())?;
      }
      _ = process_interval.tick() => {
        if let Some(status) = process.child.try_wait().map_err(|error| error.to_string())? {
          return Err(format!("kernel exited before readiness with {status}"));
        }
      }
    }
  }
}

fn correlated(envelope: &Envelope, requests: &BTreeSet<String>) -> bool {
  matches!(
    &envelope.parent_header,
    ParentHeader::Header(parent) if requests.contains(&parent.msg_id)
  )
}

fn expand_environment(
  base: &BTreeMap<OsString, OsString>,
  overrides: &BTreeMap<String, String>,
) -> Result<BTreeMap<OsString, OsString>, LaunchError> {
  let mut environment = base.clone();

  for (name, value) in overrides {
    environment
      .insert(name.into(), expand_environment_value(name, value, base)?);
  }

  Ok(environment)
}

fn expand_environment_value(
  name: &str,
  value: &str,
  base: &BTreeMap<OsString, OsString>,
) -> Result<OsString, LaunchError> {
  let bytes = value.as_bytes();
  let mut expanded = String::with_capacity(value.len());
  let mut index = 0;

  while index < bytes.len() {
    if bytes[index] != b'$' {
      let character = value[index..]
        .chars()
        .next()
        .expect("index must point to a character");
      expanded.push(character);
      index += character.len_utf8();
      continue;
    }

    let start = index;
    index += 1;

    if bytes.get(index) == Some(&b'$') {
      expanded.push('$');
      index += 1;
      continue;
    }

    let (variable, end) = if bytes.get(index) == Some(&b'{') {
      let variable_start = index + 1;
      let Some(close) = bytes[variable_start..]
        .iter()
        .position(|byte| *byte == b'}')
        .map(|offset| variable_start + offset)
      else {
        return Err(LaunchError::InvalidEnvironmentTemplate(name.into()));
      };

      (&value[variable_start..close], close + 1)
    } else {
      let variable_start = index;

      while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
      {
        index += 1;
      }

      (&value[variable_start..index], index)
    };

    if !valid_environment_name(variable) {
      return Err(LaunchError::InvalidEnvironmentTemplate(name.into()));
    }

    if let Some(replacement) = base
      .get(OsStr::new(variable))
      .and_then(|replacement| replacement.to_str())
    {
      expanded.push_str(replacement);
    } else {
      expanded.push_str(&value[start..end]);
    }

    index = end;
  }

  Ok(expanded.into())
}

fn sanitized_environment(
  inherited: &BTreeMap<OsString, OsString>,
) -> BTreeMap<OsString, OsString> {
  inherited
    .iter()
    .filter(|(name, _)| !secret_name(name))
    .map(|(name, value)| (name.clone(), value.clone()))
    .collect()
}

fn secret_name(name: &OsStr) -> bool {
  let name = name.to_string_lossy().to_ascii_uppercase();

  if name.starts_with("TAIPAN_") {
    return true;
  }

  let compact = name
    .chars()
    .filter(char::is_ascii_alphanumeric)
    .collect::<String>();
  let words = name
    .split(|character: char| !character.is_ascii_alphanumeric())
    .filter(|word| !word.is_empty())
    .collect::<Vec<_>>();
  [
    "COOKIE",
    "CREDENTIAL",
    "PASSWORD",
    "PASSWD",
    "SECRET",
    "TOKEN",
  ]
  .into_iter()
  .any(|sensitive| compact.contains(sensitive))
    || compact.contains("AUTHORIZATION")
    || compact.ends_with("PAT")
    || compact.ends_with("PWD")
    || compact.contains("APIKEY")
    || compact.contains("ACCESSKEY")
    || compact.contains("CONNECTIONSTRING")
    || compact.contains("PRIVATEKEY")
    || matches!(compact.as_str(), "DATABASEURL" | "KUBECONFIG" | "NETRC")
    || words.contains(&"AUTH")
    || words
      .last()
      .is_some_and(|word| matches!(*word, "DSN" | "PROXY" | "URI" | "URL"))
}

fn send_kernel_info(
  shell: &ChannelDriver,
  session: &str,
  requests: &mut BTreeSet<String>,
) -> Result<(), TransportError> {
  let msg_id = Uuid::new_v4().to_string();
  let envelope = Envelope {
    buffers: Vec::new(),
    content: JsonObject::new(),
    header: Header {
      date: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
      extra: JsonObject::new(),
      msg_id: msg_id.clone(),
      msg_type: MessageType::from("kernel_info_request"),
      session: session.into(),
      subshell_id: None,
      username: "taipan".into(),
      version: "5.5".into(),
    },
    identities: Vec::new(),
    metadata: JsonObject::new(),
    parent_header: ParentHeader::Empty,
  };

  shell.try_send(&envelope)?;
  requests.insert(msg_id);

  Ok(())
}

fn strip_controls(value: &str) -> String {
  value
    .chars()
    .map(|character| {
      if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
        '\u{fffd}'
      } else {
        character
      }
    })
    .collect()
}

fn startup_reason(reason: String, cleanup: Option<io::Error>) -> String {
  match cleanup {
    Some(error) => format!("{reason}; cleanup failed: {error}"),
    None => reason,
  }
}

fn truncate_utf8(value: &mut String, maximum: usize) -> bool {
  if value.len() <= maximum {
    return false;
  }

  let mut boundary = maximum;

  while !value.is_char_boundary(boundary) {
    boundary -= 1;
  }

  value.truncate(boundary);
  true
}

fn substitute_argv(argv: &[String], connection_file: &Path) -> Vec<String> {
  let connection_file = connection_file.to_string_lossy();

  argv
    .iter()
    .map(|argument| argument.replace(CONNECTION_FILE, &connection_file))
    .collect()
}

fn valid_correlated_status(
  envelope: &Envelope,
  requests: &BTreeSet<String>,
) -> bool {
  envelope.header.msg_type == MessageType::from("status")
    && correlated(envelope, requests)
    && envelope
      .content
      .get("execution_state")
      .and_then(Value::as_str)
      .is_some_and(|state| matches!(state, "busy" | "idle"))
}

fn valid_environment_name(name: &str) -> bool {
  let mut bytes = name.bytes();

  bytes
    .next()
    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
    && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn valid_iopub_welcome(envelope: &Envelope) -> bool {
  envelope.header.msg_type == MessageType::from("iopub_welcome")
    && envelope.parent_header == ParentHeader::Empty
    && envelope
      .content
      .get("subscription")
      .is_some_and(Value::is_string)
}

fn validate_kernel_info(envelope: &Envelope) -> Result<KernelInfo, String> {
  fn string(content: &JsonObject, name: &str) -> Result<String, String> {
    content
      .get(name)
      .and_then(Value::as_str)
      .filter(|value| !value.is_empty())
      .map(str::to_owned)
      .ok_or_else(|| format!("kernel_info_reply has invalid `{name}`"))
  }

  if envelope.content.get("status").and_then(Value::as_str) != Some("ok") {
    return Err("kernel_info_reply status is not ok".into());
  }

  let protocol_version = string(&envelope.content, "protocol_version")?;
  let version = protocol_version.split('.').collect::<Vec<_>>();

  if version.len() < 2
    || version
      .iter()
      .any(|component| component.parse::<u64>().is_err())
  {
    return Err("kernel_info_reply has invalid `protocol_version`".into());
  }

  let language_info = envelope
    .content
    .get("language_info")
    .and_then(Value::as_object)
    .filter(|language| {
      ["file_extension", "mimetype", "name", "version"]
        .into_iter()
        .all(|name| {
          language
            .get(name)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
        })
    })
    .cloned()
    .ok_or_else(|| {
      "kernel_info_reply has invalid `language_info`".to_string()
    })?;

  Ok(KernelInfo {
    banner: string(&envelope.content, "banner")?,
    implementation: string(&envelope.content, "implementation")?,
    implementation_version: string(
      &envelope.content,
      "implementation_version",
    )?,
    language_info,
    protocol_version,
  })
}

fn write_connection_file(
  connection: &ConnectionData,
  config: &LaunchConfig,
) -> Result<NamedTempFile, LaunchError> {
  let mut file = if let Some(runtime_dir) = &config.runtime_dir {
    Builder::new()
      .prefix("taipan-kernel-")
      .suffix(".json")
      .tempfile_in(runtime_dir)
  } else {
    Builder::new()
      .prefix("taipan-kernel-")
      .suffix(".json")
      .tempfile()
  }
  .map_err(LaunchError::ConnectionFile)?;

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    file
      .as_file()
      .set_permissions(std::fs::Permissions::from_mode(0o600))
      .map_err(LaunchError::ConnectionFile)?;
  }

  serde_json::to_writer(file.as_file_mut(), connection)
    .map_err(LaunchError::ConnectionJson)?;
  file
    .as_file_mut()
    .flush()
    .map_err(LaunchError::ConnectionFile)?;

  Ok(file)
}

#[cfg(windows)]
struct WindowsJob {
  handle: usize,
}

#[cfg(windows)]
impl WindowsJob {
  fn new(child: &Child) -> io::Result<Self> {
    let handle =
      unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };

    if handle.is_null() {
      return Err(io::Error::last_os_error());
    }

    let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    information.BasicLimitInformation.LimitFlags =
      JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let configured = unsafe {
      SetInformationJobObject(
        handle,
        JobObjectExtendedLimitInformation,
        std::ptr::from_ref(&information).cast(),
        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
      )
    };
    let Some(process) = child.raw_handle() else {
      unsafe {
        CloseHandle(handle);
      }
      return Err(io::Error::other("kernel process handle unavailable"));
    };
    let process = process as HANDLE;
    if configured == 0 {
      let error = io::Error::last_os_error();
      unsafe {
        CloseHandle(handle);
      }
      return Err(error);
    }

    if unsafe { AssignProcessToJobObject(handle, process) } == 0 {
      let error = io::Error::last_os_error();
      unsafe {
        CloseHandle(handle);
      }
      return Err(error);
    }

    Ok(Self {
      handle: handle as usize,
    })
  }

  fn terminate(&self) -> io::Result<()> {
    if unsafe { TerminateJobObject(self.handle as HANDLE, 1) } == 0 {
      Err(io::Error::last_os_error())
    } else {
      Ok(())
    }
  }
}

#[cfg(windows)]
impl Drop for WindowsJob {
  fn drop(&mut self) {
    unsafe {
      CloseHandle(self.handle as HANDLE);
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::*, std::fs};

  fn base_environment() -> BTreeMap<OsString, OsString> {
    [
      ("FOO", "foo"),
      ("PATH", "/foo/bin"),
      ("TAIPAN_SECRET", "bar"),
    ]
    .into_iter()
    .map(|(name, value)| (name.into(), value.into()))
    .collect()
  }

  #[test]
  fn argv_substitution_replaces_every_placeholder_without_parsing() {
    let argv = [
      "foo".into(),
      "--file={connection_file}".into(),
      "{connection_file}".into(),
    ];

    assert_eq!(
      substitute_argv(&argv, Path::new("/tmp/foo")),
      ["foo", "--file=/tmp/foo", "/tmp/foo"]
    );
  }

  #[test]
  fn connection_data_is_private_loopback_and_random() {
    let directory = tempfile::tempdir().unwrap();
    let config = LaunchConfig {
      runtime_dir: Some(directory.path().into()),
      ..LaunchConfig::default()
    };
    let (first, first_reservations) = ConnectionData::allocate().unwrap();
    let (second, second_reservations) = ConnectionData::allocate().unwrap();
    let file = write_connection_file(&first, &config).unwrap();
    let value =
      serde_json::from_slice::<Value>(&fs::read(file.path()).unwrap()).unwrap();

    assert_eq!(first.ip, "127.0.0.1");
    assert_eq!(first.transport, "tcp");
    assert_eq!(first.signature_scheme, "hmac-sha256");
    assert_ne!(first.key, second.key);
    assert!(first.key.len() >= 32);
    assert_eq!(value["key"], first.key);
    assert_eq!(
      [
        first.control_port,
        first.hb_port,
        first.iopub_port,
        first.shell_port,
        first.stdin_port,
      ]
      .into_iter()
      .collect::<BTreeSet<_>>()
      .len(),
      5
    );

    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;

      assert_eq!(
        fs::metadata(file.path()).unwrap().permissions().mode() & 0o777,
        0o600
      );
    }

    drop(first_reservations);
    drop(second_reservations);
  }

  #[test]
  fn environment_expands_against_base_then_overrides() {
    let base = sanitized_environment(&base_environment());
    let overrides = [
      ("BAR".into(), "${FOO}/bar".into()),
      ("FOO".into(), "override".into()),
      ("LITERAL".into(), "$$FOO".into()),
    ]
    .into_iter()
    .collect();
    let environment = expand_environment(&base, &overrides).unwrap();

    assert_eq!(environment[OsStr::new("BAR")], "foo/bar");
    assert_eq!(environment[OsStr::new("FOO")], "override");
    assert_eq!(environment[OsStr::new("LITERAL")], "$FOO");
    assert_eq!(environment[OsStr::new("PATH")], "/foo/bin");
  }

  #[test]
  fn environment_preserves_missing_variables() {
    let environment = expand_environment(
      &BTreeMap::new(),
      &[("FOO".into(), "before-${MISSING}-after".into())]
        .into_iter()
        .collect(),
    )
    .unwrap();

    assert_eq!(environment[OsStr::new("FOO")], "before-${MISSING}-after");
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn failed_startup_removes_connection_file() {
    let directory = tempfile::tempdir().unwrap();
    let spec = KernelLaunchSpec::new(
      vec!["/usr/bin/false".into(), CONNECTION_FILE.into()],
      BTreeMap::new(),
      "foo",
    );
    let result = LocalKernel::launch_with_config(
      spec,
      LaunchConfig {
        runtime_dir: Some(directory.path().into()),
        startup_timeout: Duration::from_millis(100),
        ..LaunchConfig::default()
      },
    )
    .await;

    assert!(matches!(result, Err(LaunchError::Startup { .. })));
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
  }

  #[test]
  fn inherited_application_secrets_are_excluded() {
    let inherited = [
      ("AWS_SECRET_ACCESS_KEY", "foo"),
      ("CLIENT_SECRET", "foo"),
      ("GITHUB_TOKEN", "foo"),
      ("PASSWORD", "foo"),
      ("PATH", "/foo/bin"),
      ("READONLY_DATABASE_URL", "foo"),
      ("SENTRY_DSN", "foo"),
      ("SHELL", "/bin/foo"),
      ("TAIPAN_CONFIGURATION", "foo"),
    ]
    .into_iter()
    .map(|(name, value)| (name.into(), value.into()))
    .collect();
    let environment = sanitized_environment(&inherited);

    assert_eq!(
      environment.keys().collect::<Vec<_>>(),
      [OsStr::new("PATH"), OsStr::new("SHELL")]
    );
  }

  #[test]
  fn startup_output_is_bounded_sanitized_and_redacted() {
    let spec = KernelLaunchSpec::new(Vec::new(), BTreeMap::new(), "foo");
    let connection = ConnectionData {
      control_port: 1,
      hb_port: 2,
      ip: "127.0.0.1".into(),
      iopub_port: 3,
      key: "secret-key".into(),
      shell_port: 4,
      signature_scheme: "hmac-sha256".into(),
      stdin_port: 5,
      transport: "tcp".into(),
    };
    let redactor = Redactor::new(
      &spec,
      &BTreeMap::new(),
      &BTreeMap::new(),
      &connection,
      Path::new("/private/foo.json"),
    );
    let mut capture = StartupCapture::new(40, redactor.maximum_value_length());

    capture.append(
      OutputStream::Stderr,
      b"\x1b[31msecret-key /private/foo.json\x07 trailing output trailing output trailing output",
    );
    let output = capture.output(&redactor);
    let rendered = output.to_string();

    assert!(output.truncated);
    assert!(output.stderr.len() + output.stdout.len() <= 40);
    assert!(!rendered.contains("secret-key"));
    assert!(!rendered.contains("/private/foo.json"));
    assert!(!rendered.contains('\u{1b}'));

    let mut capture = StartupCapture::new(10, redactor.maximum_value_length());
    capture.append(OutputStream::Stderr, b"xxxxxxxxxxxxxxxxxxxxxx");
    capture.append(OutputStream::Stdout, b"secret-key");
    let output = capture.output(&redactor);

    assert!(!output.stdout.contains("secret-key"));
    assert!(output.stderr.len() + output.stdout.len() <= 10);
  }
}
