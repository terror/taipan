use super::*;

const CONNECTION_FILE: &str = "{connection_file}";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
  pub heartbeat_timeout: Duration,
  pub runtime_dir: Option<PathBuf>,
  pub startup_timeout: Duration,
}

impl Default for LaunchConfig {
  fn default() -> Self {
    Self {
      heartbeat_timeout: Duration::from_secs(3),
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
  #[error("failed to spawn kernel process")]
  Spawn(#[source] io::Error),
  #[error("kernel startup failed: {0}")]
  Startup(String),
  #[error("failed to stop kernel process")]
  Stop(#[source] io::Error),
  #[error("failed to connect kernel channel")]
  Transport(#[source] TransportError),
}

#[derive(
  Clone,
  Copy,
  Debug,
  Deserialize,
  Eq,
  Hash,
  Ord,
  PartialEq,
  PartialOrd,
  Serialize,
)]
#[serde(transparent)]
pub struct KernelId(Uuid);

impl Display for KernelId {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
    self.0.fmt(formatter)
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelState {
  Busy,
  Exited,
  Failed,
  Idle,
  Starting,
  Stopping,
  Unresponsive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DocumentId(Uuid);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CellId(Uuid);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ExecutionId(Uuid);

#[derive(Clone, Debug)]
pub struct ExecutionRequest {
  pub cell_id: CellId,
  pub code: String,
  pub document_id: DocumentId,
  pub execution_id: ExecutionId,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExecutionEvent {
  pub cell_id: CellId,
  pub document_id: DocumentId,
  pub execution_id: ExecutionId,
  pub kernel_id: KernelId,
  pub message: ExecutionMessage,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ExecutionMessage {
  DisplayData {
    data: JsonObject,
    metadata: JsonObject,
  },
  Error {
    ename: String,
    evalue: String,
    traceback: Vec<String>,
  },
  ExecuteInput {
    code: String,
    execution_count: U53,
  },
  ExecuteReply {
    ename: Option<String>,
    evalue: Option<String>,
    execution_count: U53,
    status: String,
    traceback: Option<Vec<String>>,
  },
  ExecuteResult {
    data: JsonObject,
    execution_count: U53,
    metadata: JsonObject,
  },
  Status {
    execution_state: ExecutionState,
  },
  Stream {
    name: String,
    text: String,
  },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
  Busy,
  Idle,
}

#[derive(Clone, Debug)]
pub struct ManagerConfig {
  pub heartbeat_interval: Duration,
  pub launch: LaunchConfig,
  pub process_poll_interval: Duration,
  pub shutdown_timeout: Duration,
  pub terminate_timeout: Duration,
}

impl Default for ManagerConfig {
  fn default() -> Self {
    Self {
      heartbeat_interval: Duration::from_secs(1),
      launch: LaunchConfig::default(),
      process_poll_interval: Duration::from_millis(50),
      shutdown_timeout: Duration::from_secs(5),
      terminate_timeout: Duration::from_secs(2),
    }
  }
}

#[derive(Debug, Error)]
pub enum ManagerError {
  #[error("kernel {0} already has an active execution")]
  Busy(KernelId),
  #[error("kernel {0} command channel closed")]
  CommandClosed(KernelId),
  #[error("kernel {0} failed to start")]
  Failed(KernelId),
  #[error("kernel {0} does not exist")]
  NotFound(KernelId),
  #[error("kernel supervision failed")]
  Supervision(#[source] LaunchError),
  #[error("kernel supervisor task failed")]
  Task(#[source] tokio::task::JoinError),
}

pub struct LocalKernelManager {
  config: ManagerConfig,
  kernels: BTreeMap<KernelId, ManagedKernel>,
}

impl Default for LocalKernelManager {
  fn default() -> Self {
    Self::new(ManagerConfig::default())
  }
}

impl LocalKernelManager {
  #[allow(clippy::missing_errors_doc)]
  pub async fn execute(
    &self,
    id: KernelId,
    request: ExecutionRequest,
  ) -> Result<(), ManagerError> {
    let commands = self
      .kernels
      .get(&id)
      .map(|kernel| kernel.commands.clone())
      .ok_or(ManagerError::NotFound(id))?;
    let (response, result) = oneshot::channel();

    commands
      .send(SupervisorCommand::Execute { request, response })
      .await
      .map_err(|_| ManagerError::CommandClosed(id))?;

    result.await.map_err(|_| ManagerError::CommandClosed(id))?
  }

  #[must_use]
  pub fn new(config: ManagerConfig) -> Self {
    Self {
      config,
      kernels: BTreeMap::new(),
    }
  }

  #[allow(clippy::missing_errors_doc)]
  pub async fn shutdown(&mut self, id: KernelId) -> Result<(), ManagerError> {
    let kernel = self
      .kernels
      .get_mut(&id)
      .ok_or(ManagerError::NotFound(id))?;

    let _ = kernel.commands.send(SupervisorCommand::Shutdown).await;

    if let Some(task) = kernel.task.take() {
      task
        .await
        .map_err(ManagerError::Task)?
        .map_err(ManagerError::Supervision)?;
    } else if *kernel.state.borrow() == KernelState::Failed {
      return Err(ManagerError::Failed(id));
    }

    Ok(())
  }

  pub async fn shutdown_all(&mut self) {
    let ids = self.kernels.keys().copied().collect::<Vec<_>>();

    for id in ids {
      let _ = self.shutdown(id).await;
    }
  }

  #[must_use]
  pub fn start(&mut self, spec: KernelLaunchSpec) -> KernelId {
    let (events, _) = mpsc::unbounded_channel();
    self.start_with_events(spec, events)
  }

  #[must_use]
  pub fn start_with_events(
    &mut self,
    spec: KernelLaunchSpec,
    events: mpsc::UnboundedSender<ExecutionEvent>,
  ) -> KernelId {
    let id = KernelId(Uuid::new_v4());
    let (commands, command_receiver) = mpsc::channel(1);
    let (state, state_receiver) = watch::channel(KernelState::Starting);
    let config = self.config.clone();
    let task = tokio::spawn(run_supervisor(
      id,
      spec,
      config,
      state,
      command_receiver,
      events,
    ));

    self.kernels.insert(
      id,
      ManagedKernel {
        commands,
        state: state_receiver,
        task: Some(task),
      },
    );

    id
  }

  #[allow(clippy::missing_errors_doc)]
  pub fn state(&self, id: KernelId) -> Result<KernelState, ManagerError> {
    self
      .kernels
      .get(&id)
      .map(|kernel| *kernel.state.borrow())
      .ok_or(ManagerError::NotFound(id))
  }

  #[allow(clippy::missing_errors_doc)]
  pub fn subscribe_state(
    &self,
    id: KernelId,
  ) -> Result<watch::Receiver<KernelState>, ManagerError> {
    self
      .kernels
      .get(&id)
      .map(|kernel| kernel.state.clone())
      .ok_or(ManagerError::NotFound(id))
  }

  #[allow(clippy::missing_errors_doc)]
  pub async fn wait_for_start(
    &self,
    id: KernelId,
  ) -> Result<KernelState, ManagerError> {
    let mut state = self
      .kernels
      .get(&id)
      .map(|kernel| kernel.state.clone())
      .ok_or(ManagerError::NotFound(id))?;

    loop {
      let current = *state.borrow_and_update();

      match current {
        KernelState::Starting => {
          if state.changed().await.is_err() {
            return Err(ManagerError::Failed(id));
          }
        }
        KernelState::Busy | KernelState::Idle | KernelState::Unresponsive => {
          return Ok(current);
        }
        KernelState::Exited | KernelState::Failed | KernelState::Stopping => {
          return Err(ManagerError::Failed(id));
        }
      }
    }
  }
}

impl Drop for LocalKernelManager {
  fn drop(&mut self) {
    for kernel in self.kernels.values_mut() {
      let _ = kernel.commands.try_send(SupervisorCommand::Shutdown);
    }
  }
}

struct ManagedKernel {
  commands: mpsc::Sender<SupervisorCommand>,
  state: watch::Receiver<KernelState>,
  task: Option<JoinHandle<Result<(), LaunchError>>>,
}

enum SupervisorCommand {
  Execute {
    request: ExecutionRequest,
    response: oneshot::Sender<Result<(), ManagerError>>,
  },
  Shutdown,
}

#[derive(Clone, Copy)]
enum SupervisorEvent {
  Continue,
  Exited,
  Failed,
  Shutdown,
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
}

impl KernelChannels {
  fn cancel(&self) {
    self.control.cancel();
    self.heartbeat.cancel();
    self.iopub.cancel();
    self.shell.cancel();
  }

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
    } = self;

    control.cancel();
    heartbeat.cancel();
    iopub.cancel();
    shell.cancel();

    let _ = tokio::join!(
      control.shutdown(),
      heartbeat.shutdown(),
      iopub.shutdown(),
      shell.shutdown(),
    );
  }
}

pub struct LocalKernel {
  pub channels: Option<KernelChannels>,
  info: KernelInfo,
  process: Option<KernelProcess>,
  session: String,
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
    let mut environment = expand_environment(&inherited, &spec.env)?;

    #[cfg(test)]
    environment.insert(
      "TAIPAN_TEST_CONNECTION_FILE".into(),
      connection_file.path().as_os_str().into(),
    );

    if spec.language.to_ascii_lowercase().starts_with("python") {
      environment.remove(OsStr::new("PYTHONEXECUTABLE"));
    }
    let protocol = Arc::new(WireProtocol::new(connection.key.as_bytes()));
    let session = Uuid::new_v4().to_string();
    let driver_config = DriverConfig {
      client_identity: session.as_bytes().to_vec(),
      heartbeat_timeout: config.heartbeat_timeout,
      ..DriverConfig::default()
    };
    drop(reservations);

    let mut process =
      KernelProcess::spawn(&argv, &environment, connection_file)?;
    let deadline = Instant::now() + config.startup_timeout;
    let channels = time::timeout_at(deadline, async {
      loop {
        match KernelChannels::connect(
          &connection,
          protocol.clone(),
          driver_config.clone(),
        )
        .await
        {
          Ok(channels) => break Ok(channels),
          Err(TransportError::Connect(_)) => {
            time::sleep(Duration::from_millis(10)).await;
          }
          Err(error) => break Err(error),
        }
      }
    })
    .await;
    let mut channels = match channels {
      Ok(Ok(channels)) => channels,
      Ok(Err(error)) => {
        let cleanup = process.stop().await.err();
        return Err(LaunchError::Startup(startup_reason(
          error.to_string(),
          cleanup,
        )));
      }
      Err(_) => {
        let cleanup = process.stop().await.err();
        return Err(LaunchError::Startup(startup_reason(
          format!("timed out after {:?}", config.startup_timeout),
          cleanup,
        )));
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
        return Err(LaunchError::Startup(startup_reason(reason, cleanup)));
      }
      Err(_) => {
        channels.shutdown().await;
        let cleanup = process.stop().await.err();
        return Err(LaunchError::Startup(startup_reason(
          format!("timed out after {:?}", config.startup_timeout),
          cleanup,
        )));
      }
    };

    Ok(Self {
      channels: Some(channels),
      info,
      process: Some(process),
      session,
    })
  }

  #[allow(clippy::missing_errors_doc)]
  pub async fn shutdown(mut self) -> Result<(), LaunchError> {
    shutdown_kernel(
      &mut self,
      Duration::from_secs(5),
      Duration::from_secs(2),
      Duration::from_millis(50),
    )
    .await
  }
}

impl Drop for LocalKernel {
  fn drop(&mut self) {
    if let Some(channels) = &self.channels {
      channels.cancel();
    }
  }
}

async fn finish_kernel(kernel: &mut LocalKernel) -> Result<(), LaunchError> {
  if let Some(channels) = kernel.channels.take() {
    channels.shutdown().await;
  }

  if let Some(mut process) = kernel.process.take() {
    process.finish().map_err(LaunchError::Stop)?;
  }

  Ok(())
}

fn message(msg_type: &str, session: &str, content: JsonObject) -> Envelope {
  Envelope {
    buffers: Vec::new(),
    content,
    header: Header {
      date: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
      extra: JsonObject::new(),
      msg_id: Uuid::new_v4().to_string(),
      msg_type: MessageType::from(msg_type),
      session: session.into(),
      subshell_id: None,
      username: "taipan".into(),
      version: "5.5".into(),
    },
    identities: Vec::new(),
    metadata: JsonObject::new(),
    parent_header: ParentHeader::Empty,
  }
}

async fn run_supervisor(
  id: KernelId,
  spec: KernelLaunchSpec,
  config: ManagerConfig,
  state: watch::Sender<KernelState>,
  commands: mpsc::Receiver<SupervisorCommand>,
  events: mpsc::UnboundedSender<ExecutionEvent>,
) -> Result<(), LaunchError> {
  let kernel =
    LocalKernel::launch_with_config(spec, config.launch.clone()).await;
  let kernel = match kernel {
    Ok(kernel) => kernel,
    Err(error) => {
      state.send_replace(KernelState::Failed);
      return Err(error);
    }
  };

  supervise_kernel(id, kernel, config, state, commands, events).await
}

struct ActiveExecution {
  idle: bool,
  reply: bool,
  request: ExecutionRequest,
  request_message_id: String,
  running: bool,
}

impl ActiveExecution {
  fn complete(&self) -> bool {
    self.running && self.reply && self.idle
  }

  fn observe(&mut self, message: &ExecutionMessage) {
    match message {
      ExecutionMessage::Status {
        execution_state: ExecutionState::Busy,
      } => self.running = true,
      ExecutionMessage::Status {
        execution_state: ExecutionState::Idle,
      } if self.running => self.idle = true,
      ExecutionMessage::ExecuteReply { .. } => self.reply = true,
      _ => {}
    }
  }
}

#[derive(Deserialize)]
struct DisplayContent {
  data: JsonObject,
  metadata: JsonObject,
}

#[derive(Deserialize)]
struct ErrorContent {
  ename: String,
  evalue: String,
  traceback: Vec<String>,
}

#[derive(Deserialize)]
struct ExecuteInputContent {
  code: String,
  execution_count: U53,
}

#[derive(Deserialize)]
struct ExecuteReplyContent {
  ename: Option<String>,
  evalue: Option<String>,
  execution_count: U53,
  status: String,
  traceback: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ExecuteResultContent {
  data: JsonObject,
  execution_count: U53,
  metadata: JsonObject,
}

#[derive(Deserialize)]
struct StatusContent {
  execution_state: ExecutionState,
}

#[derive(Deserialize)]
struct StreamContent {
  name: String,
  text: String,
}

fn content<T: for<'de> Deserialize<'de>>(envelope: &Envelope) -> Option<T> {
  serde_json::from_value(Value::Object(envelope.content.clone())).ok()
}

fn normalize_execution_message(
  envelope: &Envelope,
) -> Option<ExecutionMessage> {
  match envelope.header.msg_type.0.as_str() {
    "display_data" => {
      let content = content::<DisplayContent>(envelope)?;
      Some(ExecutionMessage::DisplayData {
        data: content.data,
        metadata: content.metadata,
      })
    }
    "error" => {
      let content = content::<ErrorContent>(envelope)?;
      Some(ExecutionMessage::Error {
        ename: content.ename,
        evalue: content.evalue,
        traceback: content.traceback,
      })
    }
    "execute_input" => {
      let content = content::<ExecuteInputContent>(envelope)?;
      Some(ExecutionMessage::ExecuteInput {
        code: content.code,
        execution_count: content.execution_count,
      })
    }
    "execute_reply" => {
      let content = content::<ExecuteReplyContent>(envelope)?;
      Some(ExecutionMessage::ExecuteReply {
        ename: content.ename,
        evalue: content.evalue,
        execution_count: content.execution_count,
        status: content.status,
        traceback: content.traceback,
      })
    }
    "execute_result" => {
      let content = content::<ExecuteResultContent>(envelope)?;
      Some(ExecutionMessage::ExecuteResult {
        data: content.data,
        execution_count: content.execution_count,
        metadata: content.metadata,
      })
    }
    "status" => {
      let content = content::<StatusContent>(envelope)?;
      Some(ExecutionMessage::Status {
        execution_state: content.execution_state,
      })
    }
    "stream" => {
      let content = content::<StreamContent>(envelope)?;
      Some(ExecutionMessage::Stream {
        name: content.name,
        text: content.text,
      })
    }
    _ => None,
  }
}

fn correlated_request(envelope: &Envelope, request_message_id: &str) -> bool {
  matches!(
    &envelope.parent_header,
    ParentHeader::Header(parent) if parent.msg_id == request_message_id
  )
}

fn route_execution_message(
  id: KernelId,
  active: &mut Option<ActiveExecution>,
  events: &mpsc::UnboundedSender<ExecutionEvent>,
  envelope: &Envelope,
) {
  let Some(execution) = active.as_mut() else {
    return;
  };

  if !correlated_request(envelope, &execution.request_message_id) {
    return;
  }

  let Some(message) = normalize_execution_message(envelope) else {
    return;
  };

  execution.observe(&message);

  let _ = events.send(ExecutionEvent {
    cell_id: execution.request.cell_id,
    document_id: execution.request.document_id,
    execution_id: execution.request.execution_id,
    kernel_id: id,
    message,
  });

  if execution.complete() {
    *active = None;
  }
}

fn send_execute(
  kernel: &LocalKernel,
  request: ExecutionRequest,
) -> Result<ActiveExecution, TransportError> {
  let mut content = JsonObject::new();
  content.insert("allow_stdin".into(), Value::Bool(false));
  content.insert("code".into(), Value::String(request.code.clone()));
  content.insert("silent".into(), Value::Bool(false));
  content.insert("stop_on_error".into(), Value::Bool(true));
  content.insert("store_history".into(), Value::Bool(true));
  content.insert("user_expressions".into(), Value::Object(JsonObject::new()));
  let envelope = message("execute_request", &kernel.session, content);
  let request_message_id = envelope.header.msg_id.clone();

  kernel
    .channels
    .as_ref()
    .ok_or(TransportError::QueueClosed)?
    .shell
    .try_send(&envelope)?;

  Ok(ActiveExecution {
    idle: false,
    request,
    request_message_id,
    reply: false,
    running: false,
  })
}

fn send_shutdown(kernel: &LocalKernel) -> Result<String, TransportError> {
  let mut content = JsonObject::new();
  content.insert("restart".into(), Value::Bool(false));
  let request = message("shutdown_request", &kernel.session, content);
  let msg_id = request.header.msg_id.clone();

  kernel
    .channels
    .as_ref()
    .ok_or(TransportError::QueueClosed)?
    .control
    .try_send(&request)?;

  Ok(msg_id)
}

async fn shutdown_kernel(
  kernel: &mut LocalKernel,
  shutdown_timeout: Duration,
  terminate_timeout: Duration,
  process_poll_interval: Duration,
) -> Result<(), LaunchError> {
  let request = send_shutdown(kernel).ok();
  let deadline = Instant::now() + shutdown_timeout;
  let mut process_interval = time::interval(process_poll_interval);
  let mut exited = false;
  let mut shutdown_replied = false;

  while Instant::now() < deadline {
    let Some(process) = kernel.process.as_mut() else {
      exited = true;
      break;
    };

    if process
      .child
      .try_wait()
      .map_err(LaunchError::Stop)?
      .is_some()
    {
      exited = true;
      break;
    }

    let control_events = &mut kernel
      .channels
      .as_mut()
      .ok_or_else(|| {
        LaunchError::Stop(io::Error::other("channels unavailable"))
      })?
      .control_events;

    tokio::select! {
      _ = process_interval.tick() => {}
      event = control_events.recv() => {
        if let (Some(request), Some(TransportEvent::Message(reply))) =
          (&request, event)
          && reply.envelope.header.msg_type == MessageType::from("shutdown_reply")
          && matches!(
            &reply.envelope.parent_header,
            ParentHeader::Header(parent) if parent.msg_id == *request
          )
          && reply.envelope.content.get("restart").and_then(Value::as_bool)
            == Some(false)
        {
          shutdown_replied = true;
        }
      }
    }
  }

  if !exited
    && shutdown_replied
    && let Some(process) = kernel.process.as_mut()
  {
    exited = process
      .child
      .try_wait()
      .map_err(LaunchError::Stop)?
      .is_some();
  }

  if !exited && let Some(process) = kernel.process.as_mut() {
    process
      .terminate(terminate_timeout)
      .await
      .map_err(LaunchError::Stop)?;
  }

  finish_kernel(kernel).await
}

async fn supervise_kernel(
  id: KernelId,
  mut kernel: LocalKernel,
  config: ManagerConfig,
  state: watch::Sender<KernelState>,
  mut commands: mpsc::Receiver<SupervisorCommand>,
  events: mpsc::UnboundedSender<ExecutionEvent>,
) -> Result<(), LaunchError> {
  state.send_replace(KernelState::Idle);

  let mut execution_state = KernelState::Idle;
  let mut heartbeat = time::interval(config.heartbeat_interval);
  heartbeat.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
  let mut heartbeat_open = true;
  let mut heartbeat_pending = None;
  let mut process = time::interval(config.process_poll_interval);
  process.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
  let mut active_execution = None;

  loop {
    let event = {
      let channels = kernel
        .channels
        .as_mut()
        .expect("launched kernel must own channels");
      let child = &mut kernel
        .process
        .as_mut()
        .expect("launched kernel must own a process")
        .child;

      tokio::select! {
        command = commands.recv() => {
          match command {
            Some(SupervisorCommand::Execute { request, response }) => {
              let result = if active_execution.is_some() {
                Err(ManagerError::Busy(id))
              } else {
                send_execute(&kernel, request)
                  .map(|execution| active_execution = Some(execution))
                  .map_err(|_| ManagerError::CommandClosed(id))
              };

              let _ = response.send(result);
              SupervisorEvent::Continue
            }
            Some(SupervisorCommand::Shutdown) | None => SupervisorEvent::Shutdown,
          }
        }
        _ = heartbeat.tick(), if heartbeat_open && heartbeat_pending.is_none() => {
          let ping = Uuid::new_v4().as_bytes().to_vec();

          if channels.heartbeat.try_ping(ping.clone()).is_ok() {
            heartbeat_pending = Some(ping);
          } else {
            heartbeat_open = false;
            state.send_replace(KernelState::Unresponsive);
          }

          SupervisorEvent::Continue
        }
        event = channels.heartbeat_events.recv(), if heartbeat_open => {
          match event {
            Some(TransportEvent::Heartbeat(bytes))
              if heartbeat_pending.as_ref() == Some(&bytes) =>
            {
              heartbeat_pending = None;
              state.send_replace(execution_state);
            }
            Some(
              TransportEvent::Heartbeat(_)
              | TransportEvent::Error { .. }
              | TransportEvent::Message(_),
            )
            | None => {
              heartbeat_open = false;
              state.send_replace(KernelState::Unresponsive);
            }
          }

          SupervisorEvent::Continue
        }
        event = channels.iopub_events.recv() => {
          match event {
            Some(TransportEvent::Message(message)) => {
              if message.envelope.header.msg_type == MessageType::from("status") {
                execution_state = match message
                  .envelope
                  .content
                  .get("execution_state")
                  .and_then(Value::as_str)
                {
                  Some("busy") => KernelState::Busy,
                  Some("idle") => KernelState::Idle,
                  _ => execution_state,
                };

                if heartbeat_open {
                  state.send_replace(execution_state);
                }
              }

              route_execution_message(
                id,
                &mut active_execution,
                &events,
                &message.envelope,
              );
              SupervisorEvent::Continue
            }
            Some(TransportEvent::Error { .. } | TransportEvent::Heartbeat(_))
            | None => SupervisorEvent::Failed,
          }
        }
        event = channels.control_events.recv() => match event {
          Some(TransportEvent::Message(_)) => SupervisorEvent::Continue,
          Some(TransportEvent::Error { .. } | TransportEvent::Heartbeat(_))
          | None => SupervisorEvent::Failed,
        },
        event = channels.shell_events.recv() => match event {
          Some(TransportEvent::Message(message)) => {
            route_execution_message(
              id,
              &mut active_execution,
              &events,
              &message.envelope,
            );
            SupervisorEvent::Continue
          }
          Some(TransportEvent::Error { .. } | TransportEvent::Heartbeat(_))
          | None => SupervisorEvent::Failed,
        },
        _ = process.tick() => match child.try_wait() {
          Ok(Some(_)) => SupervisorEvent::Exited,
          Ok(None) => SupervisorEvent::Continue,
          Err(_) => SupervisorEvent::Failed,
        },
      }
    };

    match event {
      SupervisorEvent::Continue => {}
      SupervisorEvent::Exited => {
        let result = finish_kernel(&mut kernel).await;
        state.send_replace(if result.is_ok() {
          KernelState::Exited
        } else {
          KernelState::Failed
        });
        return result;
      }
      SupervisorEvent::Failed => {
        let exited = if let Some(process) = kernel.process.as_mut() {
          matches!(
            time::timeout(Duration::from_millis(100), process.child.wait())
              .await,
            Ok(Ok(_))
          )
        } else {
          true
        };

        if exited {
          let result = finish_kernel(&mut kernel).await;
          state.send_replace(if result.is_ok() {
            KernelState::Exited
          } else {
            KernelState::Failed
          });
          return result;
        }

        state.send_replace(KernelState::Failed);
        let cleanup = shutdown_kernel(
          &mut kernel,
          Duration::ZERO,
          config.terminate_timeout,
          config.process_poll_interval,
        )
        .await;
        cleanup?;
        return Err(LaunchError::Stop(io::Error::other(
          "kernel channel failed",
        )));
      }
      SupervisorEvent::Shutdown => {
        state.send_replace(KernelState::Stopping);
        let result = shutdown_kernel(
          &mut kernel,
          config.shutdown_timeout,
          config.terminate_timeout,
          config.process_poll_interval,
        )
        .await;
        state.send_replace(if result.is_ok() {
          KernelState::Exited
        } else {
          KernelState::Failed
        });
        return result;
      }
    }
  }
}

struct KernelProcess {
  child: Child,
  connection_file: Option<NamedTempFile>,
  #[cfg(unix)]
  process_group: Option<u32>,
  #[cfg(windows)]
  windows_job: WindowsJob,
}

impl KernelProcess {
  fn finish(&mut self) -> io::Result<()> {
    if let Some(connection_file) = self.connection_file.take() {
      connection_file.close()?;
    }

    Ok(())
  }

  fn spawn(
    argv: &[String],
    environment: &BTreeMap<OsString, OsString>,
    connection_file: NamedTempFile,
  ) -> Result<Self, LaunchError> {
    let mut command = Command::new(&argv[0]);
    command
      .args(&argv[1..])
      .env_clear()
      .envs(environment)
      .kill_on_drop(true)
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null());

    #[cfg(unix)]
    command.process_group(0);

    let child = command.spawn().map_err(LaunchError::Spawn)?;
    #[cfg(unix)]
    let process_group = child.id();
    #[cfg(windows)]
    let windows_job = WindowsJob::new(&child).map_err(LaunchError::Spawn)?;
    Ok(Self {
      child,
      connection_file: Some(connection_file),
      #[cfg(unix)]
      process_group,
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

  fn start_terminate(&mut self) -> io::Result<()> {
    #[cfg(unix)]
    {
      let Some(process_group) = self.process_group else {
        return Ok(());
      };

      match killpg(Pid::from_raw(process_group.cast_signed()), Signal::SIGTERM)
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

  async fn stop(&mut self) -> io::Result<()> {
    self.start_kill()?;
    let wait = time::timeout(Duration::from_secs(3), self.child.wait()).await;

    match wait {
      Ok(Ok(_)) => self.finish(),
      Ok(Err(error)) => Err(error),
      Err(_) => Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "kernel did not exit after termination",
      )),
    }
  }

  async fn terminate(
    &mut self,
    terminate_timeout: Duration,
  ) -> io::Result<ExitStatus> {
    self.start_terminate()?;

    if let Ok(status) =
      time::timeout(terminate_timeout, self.child.wait()).await
    {
      return status;
    }

    self.start_kill()?;

    time::timeout(Duration::from_secs(3), self.child.wait())
      .await
      .map_err(|_| {
        io::Error::new(
          io::ErrorKind::TimedOut,
          "kernel did not exit after termination",
        )
      })?
  }
}

impl Drop for KernelProcess {
  fn drop(&mut self) {
    let _ = self.start_kill();
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

fn startup_reason(reason: String, cleanup: Option<io::Error>) -> String {
  match cleanup {
    Some(error) => format!("{reason}; cleanup failed: {error}"),
    None => reason,
  }
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
  use super::*;

  static MOCK_KERNEL_LOCK: tokio::sync::Mutex<()> =
    tokio::sync::Mutex::const_new(());

  #[derive(Clone, Copy, Eq, PartialEq)]
  enum MockBehavior {
    Execute,
    Exit,
    Forced,
    Graceful,
    HeartbeatLoss,
  }

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

  async fn bind_mock(socket: &mut impl Socket, endpoint: &str) {
    time::timeout(Duration::from_secs(3), async {
      loop {
        match socket.bind(endpoint).await {
          Ok(_) => return,
          Err(ZmqError::Network(error))
            if error.kind() == io::ErrorKind::AddrInUse =>
          {
            time::sleep(Duration::from_millis(10)).await;
          }
          Err(error) => panic!("{error}"),
        }
      }
    })
    .await
    .unwrap();
  }

  fn frames_to_message(frames: Vec<Vec<u8>>) -> ZmqMessage {
    let mut frames = frames.into_iter();
    let mut message = ZmqMessage::from(frames.next().unwrap());

    for frame in frames {
      message.push_back(frame.into());
    }

    message
  }

  fn manager_config(runtime_dir: &Path) -> ManagerConfig {
    ManagerConfig {
      heartbeat_interval: Duration::from_millis(20),
      launch: LaunchConfig {
        heartbeat_timeout: Duration::from_millis(200),
        runtime_dir: Some(runtime_dir.into()),
        startup_timeout: Duration::from_secs(3),
      },
      process_poll_interval: Duration::from_millis(10),
      shutdown_timeout: Duration::from_millis(100),
      terminate_timeout: Duration::from_millis(100),
    }
  }

  fn mock_envelope(
    msg_type: &str,
    content: &Value,
    parent: Option<Header>,
    identities: Vec<Vec<u8>>,
  ) -> Envelope {
    let mut envelope =
      message(msg_type, "mock", content.as_object().unwrap().clone());
    envelope.identities = identities;
    envelope.parent_header =
      parent.map_or(ParentHeader::Empty, ParentHeader::Header);
    envelope
  }

  async fn mock_kernel(behavior: MockBehavior) {
    let path = env::var_os("TAIPAN_TEST_CONNECTION_FILE").unwrap();
    let connection = serde_json::from_slice::<ConnectionData>(
      &fs::read(PathBuf::from(path)).unwrap(),
    )
    .unwrap();
    let protocol = WireProtocol::new(connection.key.as_bytes());
    let mut control = RouterSocket::new();
    let mut heartbeat = RepSocket::new();
    let mut iopub = XPubSocket::new();
    let mut shell = RouterSocket::new();

    bind_mock(&mut control, &connection.endpoint(Channel::Control)).await;
    bind_mock(&mut heartbeat, &connection.endpoint(Channel::Heartbeat)).await;
    bind_mock(&mut iopub, &connection.endpoint(Channel::Iopub)).await;
    bind_mock(&mut shell, &connection.endpoint(Channel::Shell)).await;

    let _child = env::var_os("MOCK_CHILD_FILE").map(|path| {
      let child = StdCommand::new("sleep").arg("60").spawn().unwrap();
      fs::write(path, child.id().to_string()).unwrap();
      child
    });
    let exit_file = env::var_os("MOCK_EXIT_FILE");
    let mut heartbeat_open = true;
    let mut heartbeat_replies = 0;

    loop {
      tokio::select! {
        () = time::sleep(Duration::from_millis(10)), if behavior == MockBehavior::Exit => {
          if exit_file.as_ref().is_some_and(|path| Path::new(path).exists()) {
            return;
          }
        }
        request = control.recv() => {
          let request = protocol
            .decode(&request.unwrap().into_vec().into_iter().map(|frame| frame.to_vec()).collect::<Vec<_>>())
            .unwrap();

          if request.header.msg_type == MessageType::from("shutdown_request")
            && request.content.get("restart").and_then(Value::as_bool) == Some(false)
          {
            if let Some(path) = env::var_os("MOCK_SHUTDOWN_FILE") {
              fs::write(path, "foo").unwrap();
            }

            if behavior == MockBehavior::Graceful {
              let reply = mock_envelope(
                "shutdown_reply",
                &serde_json::json!({"restart": false, "status": "ok"}),
                Some(request.header),
                request.identities,
              );
              control
                .send(frames_to_message(protocol.encode(&reply).unwrap()))
                .await
                .unwrap();

              return;
            }
          }
        }
        request = heartbeat.recv(), if heartbeat_open => {
          let request = request.unwrap();

          if behavior == MockBehavior::HeartbeatLoss && heartbeat_replies > 0 {
            heartbeat_open = false;
          } else {
            heartbeat.send(request).await.unwrap();
            heartbeat_replies += 1;
          }
        }
        subscription = iopub.recv() => {
          subscription.unwrap();
          let welcome = mock_envelope(
            "iopub_welcome",
            &serde_json::json!({"subscription": ""}),
            None,
            vec![b"iopub_welcome".to_vec()],
          );
          iopub
            .send(frames_to_message(protocol.encode(&welcome).unwrap()))
            .await
            .unwrap();
        }
        request = shell.recv() => {
          let request = protocol
            .decode(&request.unwrap().into_vec().into_iter().map(|frame| frame.to_vec()).collect::<Vec<_>>())
            .unwrap();

          if request.header.msg_type == MessageType::from("kernel_info_request") {
            let reply = mock_envelope(
              "kernel_info_reply",
              &serde_json::json!({
                "banner": "foo",
                "implementation": "foo",
                "implementation_version": "1.0",
                "language_info": {
                  "file_extension": ".foo",
                  "mimetype": "text/foo",
                  "name": "foo",
                  "version": "1.0"
                },
                "protocol_version": "5.5",
                "status": "ok"
              }),
              Some(request.header),
              request.identities,
            );
            shell
              .send(frames_to_message(protocol.encode(&reply).unwrap()))
              .await
              .unwrap();
          } else if request.header.msg_type == MessageType::from("execute_request")
            && behavior == MockBehavior::Execute
          {
            let code = request.content["code"].as_str().unwrap();
            let parent = request.header.clone();
            let iopub_message = |msg_type: &str, content: Value| {
              mock_envelope(
                msg_type,
                &content,
                Some(parent.clone()),
                vec![msg_type.as_bytes().to_vec()],
              )
            };

            for message in [
              iopub_message(
                "status",
                serde_json::json!({"execution_state": "busy"}),
              ),
              iopub_message(
                "execute_input",
                serde_json::json!({"code": code, "execution_count": 7}),
              ),
              iopub_message(
                "stream",
                serde_json::json!({"name": "stdout", "text": "foo\n"}),
              ),
              iopub_message(
                "display_data",
                serde_json::json!({
                  "data": {"text/html": "<b>foo</b>"},
                  "metadata": {}
                }),
              ),
              iopub_message(
                "execute_result",
                serde_json::json!({
                  "data": {"text/plain": "42"},
                  "execution_count": 7,
                  "metadata": {}
                }),
              ),
            ] {
              iopub
                .send(frames_to_message(protocol.encode(&message).unwrap()))
                .await
                .unwrap();
            }

            time::sleep(Duration::from_millis(50)).await;

            let reply = mock_envelope(
              "execute_reply",
              &serde_json::json!({"execution_count": 7, "status": "ok"}),
              Some(request.header.clone()),
              request.identities.clone(),
            );
            shell
              .send(frames_to_message(protocol.encode(&reply).unwrap()))
              .await
              .unwrap();
            let idle = iopub_message(
              "status",
              serde_json::json!({"execution_state": "idle"}),
            );
            iopub
              .send(frames_to_message(protocol.encode(&idle).unwrap()))
              .await
              .unwrap();
          }
        }
      }
    }
  }

  fn mock_spec(
    behavior: MockBehavior,
    environment: impl IntoIterator<Item = (String, String)>,
  ) -> KernelLaunchSpec {
    let behavior = match behavior {
      MockBehavior::Execute => "execute",
      MockBehavior::Exit => "exit",
      MockBehavior::Forced => "forced",
      MockBehavior::Graceful => "graceful",
      MockBehavior::HeartbeatLoss => "heartbeat_loss",
    };
    let mut environment = environment.into_iter().collect::<BTreeMap<_, _>>();
    environment.insert("TAIPAN_MOCK_KERNEL".into(), behavior.into());

    KernelLaunchSpec::new(
      vec![
        env::current_exe().unwrap().to_string_lossy().into_owned(),
        "--exact".into(),
        "kernel::tests::mock_kernel_process".into(),
        "--nocapture".into(),
      ],
      environment,
      "foo",
    )
  }

  async fn wait_for_state(
    manager: &LocalKernelManager,
    id: KernelId,
    expected: KernelState,
  ) {
    time::timeout(Duration::from_secs(3), async {
      loop {
        if manager.state(id).unwrap() == expected {
          return;
        }

        time::sleep(Duration::from_millis(10)).await;
      }
    })
    .await
    .unwrap();
  }

  #[test]
  fn mock_kernel_process() {
    let Ok(behavior) = env::var("TAIPAN_MOCK_KERNEL") else {
      return;
    };
    let behavior = match behavior.as_str() {
      "execute" => MockBehavior::Execute,
      "exit" => MockBehavior::Exit,
      "forced" => MockBehavior::Forced,
      "graceful" => MockBehavior::Graceful,
      "heartbeat_loss" => MockBehavior::HeartbeatLoss,
      _ => panic!("invalid mock behavior"),
    };

    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap()
      .block_on(mock_kernel(behavior));
  }

  #[tokio::test]
  async fn manager_confirms_process_exit() {
    let _guard = MOCK_KERNEL_LOCK.lock().await;
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let exit_file = runtime.path().join("foo");
    let id = manager.start(mock_spec(
      MockBehavior::Exit,
      [(
        "MOCK_EXIT_FILE".into(),
        exit_file.to_string_lossy().into_owned(),
      )],
    ));

    let result = manager.wait_for_start(id).await;
    assert!(
      result.is_ok(),
      "{result:?}: {:?}",
      manager.shutdown(id).await
    );
    fs::write(&exit_file, "bar").unwrap();
    wait_for_state(&manager, id, KernelState::Exited).await;
    manager.shutdown(id).await.unwrap();
    fs::remove_file(exit_file).unwrap();

    assert_eq!(fs::read_dir(runtime.path()).unwrap().count(), 0);
  }

  #[tokio::test]
  async fn manager_routes_one_correlated_execution() {
    let _guard = MOCK_KERNEL_LOCK.lock().await;
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let (events, mut event_receiver) = mpsc::unbounded_channel();
    let id =
      manager.start_with_events(mock_spec(MockBehavior::Execute, []), events);

    manager.wait_for_start(id).await.unwrap();
    manager.execute(id, execution_request()).await.unwrap();

    assert!(matches!(
      manager.execute(id, execution_request()).await,
      Err(ManagerError::Busy(error_id)) if error_id == id
    ));

    let received = time::timeout(Duration::from_secs(3), async {
      let mut messages = Vec::new();

      loop {
        let event = event_receiver.recv().await.unwrap();
        assert_eq!(event.kernel_id, id);
        assert_eq!(event.cell_id, execution_request().cell_id);
        assert_eq!(event.document_id, execution_request().document_id);
        assert_eq!(event.execution_id, execution_request().execution_id);
        let complete = matches!(
          event.message,
          ExecutionMessage::Status {
            execution_state: ExecutionState::Idle
          }
        );
        messages.push(event.message);

        if complete {
          return messages;
        }
      }
    })
    .await
    .unwrap();

    assert!(matches!(
      received.as_slice(),
      [
        ExecutionMessage::Status {
          execution_state: ExecutionState::Busy
        },
        ExecutionMessage::ExecuteInput { .. },
        ExecutionMessage::Stream { .. },
        ExecutionMessage::DisplayData { .. },
        ExecutionMessage::ExecuteResult { .. },
        ExecutionMessage::ExecuteReply { .. },
        ExecutionMessage::Status {
          execution_state: ExecutionState::Idle
        },
      ]
    ));

    manager.shutdown(id).await.unwrap();
  }

  #[tokio::test]
  async fn manager_drop_cleans_up() {
    let _guard = MOCK_KERNEL_LOCK.lock().await;
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("foo");
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let id = manager.start(mock_spec(
      MockBehavior::Graceful,
      [("MOCK_SHUTDOWN_FILE".into(), marker.display().to_string())],
    ));

    manager.wait_for_start(id).await.unwrap();
    drop(manager);

    time::timeout(Duration::from_secs(3), async {
      while !marker.exists()
        || fs::read_dir(runtime.path()).unwrap().next().is_some()
      {
        time::sleep(Duration::from_millis(10)).await;
      }
    })
    .await
    .unwrap();
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn manager_failed_startup_is_terminal() {
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let id = manager.start(KernelLaunchSpec::new(
      vec!["/usr/bin/false".into()],
      BTreeMap::new(),
      "foo",
    ));

    assert!(matches!(
      manager.wait_for_start(id).await,
      Err(ManagerError::Failed(error_id)) if error_id == id
    ));
    assert_eq!(manager.state(id).unwrap(), KernelState::Failed);
    assert!(matches!(
      manager.shutdown(id).await,
      Err(ManagerError::Supervision(LaunchError::Startup(_)))
    ));
    assert_eq!(fs::read_dir(runtime.path()).unwrap().count(), 0);
  }

  #[tokio::test]
  async fn manager_gracefully_shuts_down_once() {
    let _guard = MOCK_KERNEL_LOCK.lock().await;
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("foo");
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let id = manager.start(mock_spec(
      MockBehavior::Graceful,
      [("MOCK_SHUTDOWN_FILE".into(), marker.display().to_string())],
    ));

    assert_eq!(manager.state(id).unwrap(), KernelState::Starting);
    assert_eq!(manager.wait_for_start(id).await.unwrap(), KernelState::Idle);
    manager.shutdown(id).await.unwrap();
    manager.shutdown(id).await.unwrap();

    assert_eq!(manager.state(id).unwrap(), KernelState::Exited);
    assert!(marker.exists());
    assert_eq!(fs::read_dir(runtime.path()).unwrap().count(), 0);
  }

  #[tokio::test]
  async fn manager_marks_heartbeat_loss_unresponsive() {
    let _guard = MOCK_KERNEL_LOCK.lock().await;
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let id = manager.start(mock_spec(MockBehavior::HeartbeatLoss, []));

    manager.wait_for_start(id).await.unwrap();
    wait_for_state(&manager, id, KernelState::Unresponsive).await;
    manager.shutdown(id).await.unwrap();

    assert_eq!(manager.state(id).unwrap(), KernelState::Exited);
    assert_eq!(fs::read_dir(runtime.path()).unwrap().count(), 0);
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn manager_timeout_terminates_child_process() {
    use nix::sys::signal::kill;

    let _guard = MOCK_KERNEL_LOCK.lock().await;
    let directory = tempfile::tempdir().unwrap();
    let child_file = directory.path().join("foo");
    let shutdown_file = directory.path().join("bar");
    let runtime = tempfile::tempdir().unwrap();
    let mut manager = LocalKernelManager::new(manager_config(runtime.path()));
    let id = manager.start(mock_spec(
      MockBehavior::Forced,
      [
        ("MOCK_CHILD_FILE".into(), child_file.display().to_string()),
        (
          "MOCK_SHUTDOWN_FILE".into(),
          shutdown_file.display().to_string(),
        ),
      ],
    ));

    manager.wait_for_start(id).await.unwrap();
    time::timeout(Duration::from_secs(3), async {
      while !child_file.exists() {
        time::sleep(Duration::from_millis(10)).await;
      }
    })
    .await
    .unwrap();
    let child = fs::read_to_string(&child_file)
      .unwrap()
      .parse::<i32>()
      .unwrap();

    manager.shutdown(id).await.unwrap();

    time::timeout(Duration::from_secs(3), async {
      while kill(Pid::from_raw(child), None).is_ok() {
        time::sleep(Duration::from_millis(10)).await;
      }
    })
    .await
    .unwrap();
    assert!(shutdown_file.exists());
    assert_eq!(manager.state(id).unwrap(), KernelState::Exited);
    assert_eq!(fs::read_dir(runtime.path()).unwrap().count(), 0);
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
  fn environment_inherits_base_then_expands_overrides() {
    let base = base_environment();
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
    assert_eq!(environment[OsStr::new("TAIPAN_SECRET")], "bar");
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

    assert!(matches!(result, Err(LaunchError::Startup(_))));
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
  }

  fn execution_request() -> ExecutionRequest {
    ExecutionRequest {
      cell_id: CellId(Uuid::from_u128(1)),
      code: "foo".into(),
      document_id: DocumentId(Uuid::from_u128(2)),
      execution_id: ExecutionId(Uuid::from_u128(3)),
    }
  }

  fn active_execution() -> ActiveExecution {
    ActiveExecution {
      idle: false,
      request: execution_request(),
      request_message_id: "foo".into(),
      reply: false,
      running: false,
    }
  }

  #[test]
  fn execution_completes_after_busy_reply_and_idle_in_either_order() {
    fn check(messages: &[ExecutionMessage; 2]) {
      let mut execution = active_execution();
      execution.observe(&ExecutionMessage::Status {
        execution_state: ExecutionState::Busy,
      });

      assert!(!execution.complete());

      execution.observe(&messages[0]);
      assert!(!execution.complete());

      execution.observe(&messages[1]);
      assert!(execution.complete());
    }

    let reply = || ExecutionMessage::ExecuteReply {
      ename: None,
      evalue: None,
      execution_count: U53::from(7_u8),
      status: "ok".into(),
      traceback: None,
    };
    let idle = || ExecutionMessage::Status {
      execution_state: ExecutionState::Idle,
    };

    check(&[reply(), idle()]);
    check(&[idle(), reply()]);
  }

  #[test]
  fn execution_messages_are_normalized_without_transient_data() {
    #[track_caller]
    fn case(msg_type: &str, content: &Value, expected: ExecutionMessage) {
      let envelope = mock_envelope(msg_type, content, None, Vec::new());
      assert_eq!(normalize_execution_message(&envelope), Some(expected));
    }

    case(
      "stream",
      &serde_json::json!({"name": "stdout", "text": "foo\n"}),
      ExecutionMessage::Stream {
        name: "stdout".into(),
        text: "foo\n".into(),
      },
    );
    case(
      "display_data",
      &serde_json::json!({
        "data": {"text/html": "<b>foo</b>"},
        "metadata": {"foo": true},
        "transient": {"display_id": "secret"}
      }),
      ExecutionMessage::DisplayData {
        data: serde_json::json!({"text/html": "<b>foo</b>"})
          .as_object()
          .unwrap()
          .clone(),
        metadata: serde_json::json!({"foo": true})
          .as_object()
          .unwrap()
          .clone(),
      },
    );
    case(
      "execute_result",
      &serde_json::json!({
        "data": {"text/plain": "42"},
        "execution_count": 7,
        "metadata": {}
      }),
      ExecutionMessage::ExecuteResult {
        data: serde_json::json!({"text/plain": "42"})
          .as_object()
          .unwrap()
          .clone(),
        execution_count: U53::from(7_u8),
        metadata: JsonObject::new(),
      },
    );
    case(
      "error",
      &serde_json::json!({
        "ename": "FooError",
        "evalue": "bar",
        "traceback": ["baz"]
      }),
      ExecutionMessage::Error {
        ename: "FooError".into(),
        evalue: "bar".into(),
        traceback: vec!["baz".into()],
      },
    );
    case(
      "execute_input",
      &serde_json::json!({"code": "foo", "execution_count": 7}),
      ExecutionMessage::ExecuteInput {
        code: "foo".into(),
        execution_count: U53::from(7_u8),
      },
    );
    case(
      "status",
      &serde_json::json!({"execution_state": "busy"}),
      ExecutionMessage::Status {
        execution_state: ExecutionState::Busy,
      },
    );
    case(
      "execute_reply",
      &serde_json::json!({"execution_count": 7, "status": "ok"}),
      ExecutionMessage::ExecuteReply {
        ename: None,
        evalue: None,
        execution_count: U53::from(7_u8),
        status: "ok".into(),
        traceback: None,
      },
    );
  }
}
