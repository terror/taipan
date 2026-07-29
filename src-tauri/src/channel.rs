use {
  crate::wire::{Channel, Envelope, Frame, WireError, WireProtocol},
  futures::{StreamExt, channel::mpsc as monitor},
  std::{sync::Arc, time::Duration},
  thiserror::Error,
  tokio::{
    sync::{mpsc, watch},
    task::{JoinError, JoinHandle},
    time,
  },
  zeromq::{
    DealerRecvHalf, DealerSendHalf, DealerSocket, ReqSocket, Socket,
    SocketEvent, SocketOptions, SocketRecv, SocketSend, SubSocket, ZmqError,
    ZmqMessage, util::PeerIdentity,
  },
};

#[derive(Debug)]
pub struct ChannelMessage {
  pub channel: Channel,
  pub envelope: Envelope,
}

pub struct ChannelDriver {
  cancellation: watch::Sender<bool>,
  channel: Channel,
  commands: mpsc::Sender<ZmqMessage>,
  config: DriverConfig,
  protocol: Arc<WireProtocol>,
  task: Option<JoinHandle<()>>,
}

#[allow(clippy::missing_errors_doc)]
impl ChannelDriver {
  pub fn cancel(&self) {
    self.cancellation.send_replace(true);
  }

  #[must_use]
  pub fn channel(&self) -> Channel {
    self.channel
  }

  pub async fn connect(
    channel: Channel,
    endpoint: &str,
    protocol: Arc<WireProtocol>,
    config: DriverConfig,
  ) -> Result<(Self, mpsc::Receiver<TransportEvent>), TransportError> {
    config.validate()?;

    if channel == Channel::Heartbeat {
      return Err(TransportError::InvalidChannel(channel));
    }

    let (cancellation, cancelled) = watch::channel(false);
    let (commands, command_receiver) = mpsc::channel(config.queue_capacity);
    let (events, event_receiver) = mpsc::channel(config.queue_capacity);

    let task = match channel {
      Channel::Iopub => {
        let mut socket = SubSocket::with_options(config.socket_options()?);

        let monitor = socket.monitor();

        socket.connect(endpoint).await.map_err(connect_error)?;
        socket.subscribe("").await.map_err(connect_error)?;

        tokio::spawn(run_subscriber(
          channel,
          config.clone(),
          protocol.clone(),
          socket,
          monitor,
          events,
          cancelled,
        ))
      }
      Channel::Control | Channel::Shell | Channel::Stdin => {
        let mut socket = DealerSocket::with_options(config.socket_options()?);

        let monitor = socket.monitor();

        socket.connect(endpoint).await.map_err(connect_error)?;

        let (sender, receiver) = socket.split();

        tokio::spawn(run_dealer(DealerTask {
          cancelled,
          channel,
          commands: command_receiver,
          config: config.clone(),
          events,
          monitor,
          protocol: protocol.clone(),
          receiver,
          sender,
        }))
      }
      Channel::Heartbeat => unreachable!(),
    };

    Ok((
      Self {
        cancellation,
        channel,
        commands,
        config,
        protocol,
        task: Some(task),
      },
      event_receiver,
    ))
  }

  pub async fn shutdown(mut self) -> Result<(), TransportError> {
    self.cancel();

    if let Some(task) = self.task.take() {
      task.await?;
    }

    Ok(())
  }

  pub fn try_send(&self, envelope: &Envelope) -> Result<(), TransportError> {
    if self.channel == Channel::Iopub {
      return Err(TransportError::InvalidChannel(self.channel));
    }

    let frames = self
      .protocol
      .encode(envelope)
      .map_err(TransportError::Encode)?;

    validate_frames(&frames, &self.config)?;

    let message = frames_to_message(frames)?;

    self
      .commands
      .try_send(message)
      .map_err(|error| match error {
        mpsc::error::TrySendError::Closed(_) => TransportError::QueueClosed,
        mpsc::error::TrySendError::Full(_) => TransportError::QueueFull,
      })
  }
}

impl Drop for ChannelDriver {
  fn drop(&mut self) {
    self.cancellation.send_replace(true);
  }
}

#[derive(Clone, Debug)]
pub struct DriverConfig {
  pub client_identity: Vec<u8>,
  pub connect_timeout: Duration,
  pub heartbeat_timeout: Duration,
  pub max_frame_bytes: usize,
  pub max_message_bytes: usize,
  pub max_message_frames: usize,
  pub queue_capacity: usize,
}

impl Default for DriverConfig {
  fn default() -> Self {
    Self {
      client_identity: PeerIdentity::new().into(),
      connect_timeout: Duration::from_secs(10),
      heartbeat_timeout: Duration::from_secs(3),
      max_frame_bytes: 16 * 1024 * 1024,
      max_message_bytes: 64 * 1024 * 1024,
      max_message_frames: 1_024,
      queue_capacity: 256,
    }
  }
}

impl DriverConfig {
  fn socket_options(&self) -> Result<SocketOptions, TransportError> {
    let identity = PeerIdentity::try_from(self.client_identity.clone())
      .map_err(|_| TransportError::InvalidConfig)?;

    let mut options = SocketOptions::default();

    options.connect_timeout(self.connect_timeout);
    options.peer_identity(identity);

    Ok(options)
  }

  fn validate(&self) -> Result<(), TransportError> {
    if self.client_identity.is_empty()
      || self.client_identity.len() > PeerIdentity::MAX_LENGTH
      || self.connect_timeout.is_zero()
      || self.heartbeat_timeout.is_zero()
      || self.max_frame_bytes == 0
      || self.max_message_bytes < self.max_frame_bytes
      || self.max_message_frames == 0
      || self.queue_capacity == 0
    {
      Err(TransportError::InvalidConfig)
    } else {
      Ok(())
    }
  }
}

pub struct HeartbeatDriver {
  cancellation: watch::Sender<bool>,
  commands: mpsc::Sender<Vec<u8>>,
  config: DriverConfig,
  task: Option<JoinHandle<()>>,
}

#[allow(clippy::missing_errors_doc)]
impl HeartbeatDriver {
  pub fn cancel(&self) {
    self.cancellation.send_replace(true);
  }

  pub async fn connect(
    endpoint: &str,
    config: DriverConfig,
  ) -> Result<(Self, mpsc::Receiver<TransportEvent>), TransportError> {
    config.validate()?;

    let (cancellation, cancelled) = watch::channel(false);
    let (commands, command_receiver) = mpsc::channel(config.queue_capacity);
    let (events, event_receiver) = mpsc::channel(config.queue_capacity);

    let mut socket = ReqSocket::with_options(config.socket_options()?);

    let monitor = socket.monitor();

    socket.connect(endpoint).await.map_err(connect_error)?;

    let task = tokio::spawn(run_heartbeat(
      config.clone(),
      socket,
      monitor,
      command_receiver,
      events,
      cancelled,
    ));

    Ok((
      Self {
        cancellation,
        commands,
        config,
        task: Some(task),
      },
      event_receiver,
    ))
  }

  pub async fn shutdown(mut self) -> Result<(), TransportError> {
    self.cancel();

    if let Some(task) = self.task.take() {
      task.await?;
    }

    Ok(())
  }

  pub fn try_ping(&self, bytes: Vec<u8>) -> Result<(), TransportError> {
    validate_frame(&bytes, &self.config)?;

    self.commands.try_send(bytes).map_err(|error| match error {
      mpsc::error::TrySendError::Closed(_) => TransportError::QueueClosed,
      mpsc::error::TrySendError::Full(_) => TransportError::QueueFull,
    })
  }
}

impl Drop for HeartbeatDriver {
  fn drop(&mut self) {
    self.cancellation.send_replace(true);
  }
}

#[derive(Debug, Error)]
pub enum TransportError {
  #[error("failed to connect ZeroMQ socket: {0}")]
  Connect(#[source] ZmqError),
  #[error("failed to decode Jupyter message: {0}")]
  Decode(#[source] WireError),
  #[error("ZeroMQ peer disconnected")]
  Disconnected,
  #[error("ZeroMQ message cannot be empty")]
  EmptyMessage,
  #[error("failed to encode Jupyter message: {0}")]
  Encode(#[source] WireError),
  #[error("frame is {actual} bytes, maximum is {maximum}")]
  FrameTooLarge { actual: usize, maximum: usize },
  #[error("heartbeat reply did not echo the request bytes")]
  HeartbeatMismatch,
  #[error("channel {0} does not support this driver operation")]
  InvalidChannel(Channel),
  #[error(
    "driver limits and timeouts must be nonzero and internally consistent"
  )]
  InvalidConfig,
  #[error("message is {actual} bytes, maximum is {maximum}")]
  MessageTooLarge { actual: usize, maximum: usize },
  #[error("driver queue is closed")]
  QueueClosed,
  #[error("driver queue is full")]
  QueueFull,
  #[error("failed to receive from ZeroMQ socket: {0}")]
  Receive(#[source] ZmqError),
  #[error("failed to send on ZeroMQ socket: {0}")]
  Send(#[source] ZmqError),
  #[error("Jupyter message signature failed verification: {0}")]
  Signature(#[source] WireError),
  #[error("channel task failed: {0}")]
  Task(#[from] JoinError),
  #[error("transport timed out after {0:?}")]
  Timeout(Duration),
  #[error("message has {actual} frames, maximum is {maximum}")]
  TooManyFrames { actual: usize, maximum: usize },
}

#[derive(Debug)]
pub enum TransportEvent {
  Error {
    channel: Channel,
    error: TransportError,
  },
  Heartbeat(Vec<u8>),
  Message(Box<ChannelMessage>),
}

struct DealerTask {
  cancelled: watch::Receiver<bool>,
  channel: Channel,
  commands: mpsc::Receiver<ZmqMessage>,
  config: DriverConfig,
  events: mpsc::Sender<TransportEvent>,
  monitor: monitor::Receiver<SocketEvent>,
  protocol: Arc<WireProtocol>,
  receiver: DealerRecvHalf,
  sender: DealerSendHalf,
}

fn connect_error(error: ZmqError) -> TransportError {
  match error {
    ZmqError::ConnectTimeout(duration) => TransportError::Timeout(duration),
    error => TransportError::Connect(error),
  }
}

fn decode_error(error: WireError) -> TransportError {
  match error {
    WireError::BadSignature | WireError::InvalidSignatureEncoding => {
      TransportError::Signature(error)
    }
    error => TransportError::Decode(error),
  }
}

async fn emit(
  events: &mpsc::Sender<TransportEvent>,
  cancelled: &mut watch::Receiver<bool>,
  event: TransportEvent,
) -> bool {
  tokio::select! {
    biased;
    _ = cancelled.changed() => false,
    result = events.send(event) => result.is_ok(),
  }
}

async fn emit_message(
  channel: Channel,
  config: &DriverConfig,
  protocol: &WireProtocol,
  message: ZmqMessage,
  events: &mpsc::Sender<TransportEvent>,
  cancelled: &mut watch::Receiver<bool>,
) -> bool {
  let event = match message_to_frames(message, config)
    .and_then(|frames| protocol.decode(&frames).map_err(decode_error))
  {
    Ok(envelope) => {
      TransportEvent::Message(Box::new(ChannelMessage { channel, envelope }))
    }
    Err(error) => TransportEvent::Error { channel, error },
  };

  emit(events, cancelled, event).await
}

fn frames_to_message(frames: Vec<Frame>) -> Result<ZmqMessage, TransportError> {
  let mut frames = frames.into_iter();

  let first = frames.next().ok_or(TransportError::EmptyMessage)?;

  let mut message = ZmqMessage::from(first);

  for frame in frames {
    message.push_back(frame.into());
  }

  Ok(message)
}

fn message_to_frames(
  message: ZmqMessage,
  config: &DriverConfig,
) -> Result<Vec<Frame>, TransportError> {
  validate_lengths(message.iter().map(AsRef::as_ref), config)?;
  Ok(
    message
      .into_vec()
      .into_iter()
      .map(|frame| frame.to_vec())
      .collect(),
  )
}

fn receive_error(error: ZmqError) -> TransportError {
  match error {
    ZmqError::ConnectTimeout(duration) => TransportError::Timeout(duration),
    ZmqError::NoMessage | ZmqError::Other("Server disconnected") => {
      TransportError::Disconnected
    }
    error => TransportError::Receive(error),
  }
}

async fn run_dealer(task: DealerTask) {
  let DealerTask {
    mut cancelled,
    channel,
    mut commands,
    config,
    events,
    mut monitor,
    protocol,
    mut receiver,
    mut sender,
  } = task;
  let mut monitoring = true;

  loop {
    tokio::select! {
      biased;
      _ = cancelled.changed() => break,
      command = commands.recv() => {
        let Some(command) = command else {
          break;
        };

        let result = tokio::select! {
          biased;
          _ = cancelled.changed() => break,
          result = sender.send(command) => result,
        };

        if let Err(error) = result {
          let error = TransportEvent::Error {
            channel,
            error: send_error(error),
          };

          emit(&events, &mut cancelled, error).await;

          break;
        }
      }
      result = receiver.recv() => match result {
        Ok(message) => {
          if !emit_message(
            channel,
            &config,
            &protocol,
            message,
            &events,
            &mut cancelled,
          ).await {
            break;
          }
        }
        Err(error) => {
          let error = TransportEvent::Error {
            channel,
            error: receive_error(error),
          };

          emit(&events, &mut cancelled, error).await;

          break;
        }
      },
      event = monitor.next(), if monitoring => match event {
        Some(SocketEvent::Closed | SocketEvent::Disconnected(_)) => {
          let error = TransportEvent::Error {
            channel,
            error: TransportError::Disconnected,
          };

          emit(&events, &mut cancelled, error).await;

          break;
        }
        Some(_) => {}
        None => monitoring = false,
      },
    }
  }
}

async fn run_heartbeat(
  config: DriverConfig,
  mut socket: ReqSocket,
  mut monitor: monitor::Receiver<SocketEvent>,
  mut commands: mpsc::Receiver<Vec<u8>>,
  events: mpsc::Sender<TransportEvent>,
  mut cancelled: watch::Receiver<bool>,
) {
  let channel = Channel::Heartbeat;

  let mut monitoring = true;

  loop {
    let ping = tokio::select! {
      biased;
      _ = cancelled.changed() => break,
      command = commands.recv() => {
        let Some(command) = command else {
          break;
        };

        command
      }
      event = monitor.next(), if monitoring => match event {
        Some(SocketEvent::Closed | SocketEvent::Disconnected(_)) => {
          let error = TransportEvent::Error {
            channel,
            error: TransportError::Disconnected,
          };

          emit(&events, &mut cancelled, error).await;

          break;
        }
        Some(_) => continue,
        None => {
          monitoring = false;
          continue;
        }
      },
    };

    let result = tokio::select! {
      biased;
      _ = cancelled.changed() => break,
      result = socket.send(ZmqMessage::from(ping.clone())) => result,
    };

    if let Err(error) = result {
      let error = TransportEvent::Error {
        channel,
        error: send_error(error),
      };

      emit(&events, &mut cancelled, error).await;

      break;
    }

    let result = tokio::select! {
      biased;
      _ = cancelled.changed() => break,
      result = time::timeout(config.heartbeat_timeout, socket.recv()) => result,
    };

    let event = match result {
      Err(_) => TransportEvent::Error {
        channel,
        error: TransportError::Timeout(config.heartbeat_timeout),
      },
      Ok(Err(error)) => TransportEvent::Error {
        channel,
        error: receive_error(error),
      },
      Ok(Ok(message)) => match message_to_frames(message, &config) {
        Ok(frames) if frames == [ping.clone()] => {
          TransportEvent::Heartbeat(ping)
        }
        Ok(_) => TransportEvent::Error {
          channel,
          error: TransportError::HeartbeatMismatch,
        },
        Err(error) => TransportEvent::Error { channel, error },
      },
    };

    let stop = matches!(
      event,
      TransportEvent::Error {
        error: TransportError::Disconnected | TransportError::Timeout(_),
        ..
      }
    );

    if !emit(&events, &mut cancelled, event).await || stop {
      break;
    }
  }
}

async fn run_subscriber(
  channel: Channel,
  config: DriverConfig,
  protocol: Arc<WireProtocol>,
  mut socket: SubSocket,
  mut monitor: monitor::Receiver<SocketEvent>,
  events: mpsc::Sender<TransportEvent>,
  mut cancelled: watch::Receiver<bool>,
) {
  let mut monitoring = true;

  loop {
    tokio::select! {
      biased;
      _ = cancelled.changed() => break,
      result = socket.recv() => match result {
        Ok(message) => {
          if !emit_message(
            channel,
            &config,
            &protocol,
            message,
            &events,
            &mut cancelled,
          ).await {
            break;
          }
        }
        Err(error) => {
          let error = TransportEvent::Error {
            channel,
            error: receive_error(error),
          };

          emit(&events, &mut cancelled, error).await;

          break;
        }
      },
      event = monitor.next(), if monitoring => match event {
        Some(SocketEvent::Closed | SocketEvent::Disconnected(_)) => {
          let error = TransportEvent::Error {
            channel,
            error: TransportError::Disconnected,
          };

          emit(&events, &mut cancelled, error).await;

          break;
        }
        Some(_) => {}
        None => monitoring = false,
      },
    }
  }
}

fn send_error(error: ZmqError) -> TransportError {
  match error {
    ZmqError::ConnectTimeout(duration) => TransportError::Timeout(duration),
    ZmqError::ReturnToSender { .. } => TransportError::Disconnected,
    error => TransportError::Send(error),
  }
}

fn validate_frame(
  frame: &[u8],
  config: &DriverConfig,
) -> Result<(), TransportError> {
  validate_lengths([frame], config)
}

fn validate_frames(
  frames: &[Frame],
  config: &DriverConfig,
) -> Result<(), TransportError> {
  validate_lengths(frames.iter().map(Vec::as_slice), config)
}

fn validate_lengths<'a>(
  frames: impl IntoIterator<Item = &'a [u8]>,
  config: &DriverConfig,
) -> Result<(), TransportError> {
  let mut count = 0_usize;
  let mut total = 0_usize;

  for frame in frames {
    count += 1;

    if count > config.max_message_frames {
      return Err(TransportError::TooManyFrames {
        actual: count,
        maximum: config.max_message_frames,
      });
    }

    if frame.len() > config.max_frame_bytes {
      return Err(TransportError::FrameTooLarge {
        actual: frame.len(),
        maximum: config.max_frame_bytes,
      });
    }

    total = total.checked_add(frame.len()).ok_or(
      TransportError::MessageTooLarge {
        actual: usize::MAX,
        maximum: config.max_message_bytes,
      },
    )?;

    if total > config.max_message_bytes {
      return Err(TransportError::MessageTooLarge {
        actual: total,
        maximum: config.max_message_bytes,
      });
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    crate::wire::{DELIMITER, Header, JsonObject, MessageType, ParentHeader},
    zeromq::{RepSocket, RouterSocket, XPubSocket},
  };

  async fn event(
    events: &mut mpsc::Receiver<TransportEvent>,
  ) -> TransportEvent {
    time::timeout(Duration::from_secs(2), events.recv())
      .await
      .unwrap()
      .unwrap()
  }

  fn envelope() -> Envelope {
    Envelope {
      buffers: vec![b"foo".to_vec(), b"\0\xff".to_vec()],
      content: JsonObject::new(),
      header: Header {
        date: "foo".into(),
        extra: JsonObject::new(),
        msg_id: "foo".into(),
        msg_type: MessageType::from("foo"),
        session: "foo".into(),
        subshell_id: None,
        username: "foo".into(),
        version: "5.5".into(),
      },
      identities: Vec::new(),
      metadata: JsonObject::new(),
      parent_header: ParentHeader::Empty,
    }
  }

  fn protocol() -> Arc<WireProtocol> {
    Arc::new(WireProtocol::new(b"foo", "hmac-sha256").unwrap())
  }

  #[tokio::test]
  async fn cancellation_stops_blocked_tasks() {
    let mut peer = RouterSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let (driver, _events) = ChannelDriver::connect(
      Channel::Shell,
      &endpoint.to_string(),
      protocol(),
      DriverConfig::default(),
    )
    .await
    .unwrap();

    time::timeout(Duration::from_secs(1), driver.shutdown())
      .await
      .unwrap()
      .unwrap();

    let mut peer = RepSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let (driver, _events) =
      HeartbeatDriver::connect(&endpoint.to_string(), DriverConfig::default())
        .await
        .unwrap();
    driver.try_ping(b"foo".to_vec()).unwrap();
    peer.recv().await.unwrap();

    time::timeout(Duration::from_secs(1), driver.shutdown())
      .await
      .unwrap()
      .unwrap();
  }

  #[tokio::test]
  async fn cross_channel_stdin_uses_shell_identity() {
    let mut shell_peer = RouterSocket::new();
    let shell_endpoint = shell_peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let mut stdin_peer = RouterSocket::new();
    let stdin_endpoint = stdin_peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let config = DriverConfig::default();
    let protocol = protocol();
    let (shell, _shell_events) = ChannelDriver::connect(
      Channel::Shell,
      &shell_endpoint.to_string(),
      protocol.clone(),
      config.clone(),
    )
    .await
    .unwrap();
    let (stdin, mut stdin_events) = ChannelDriver::connect(
      Channel::Stdin,
      &stdin_endpoint.to_string(),
      protocol.clone(),
      config.clone(),
    )
    .await
    .unwrap();

    shell.try_send(&envelope()).unwrap();
    let request = shell_peer.recv().await.unwrap();
    assert_eq!(request.get(0).unwrap().as_ref(), config.client_identity);

    let mut input_request = envelope();
    input_request.identities.push(config.client_identity);
    stdin_peer
      .send(
        frames_to_message(protocol.encode(&input_request).unwrap()).unwrap(),
      )
      .await
      .unwrap();

    let TransportEvent::Message(message) = event(&mut stdin_events).await
    else {
      panic!("expected message event");
    };
    assert_eq!(message.channel, Channel::Stdin);

    shell.shutdown().await.unwrap();
    stdin.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn dealer_channels_round_trip_messages() {
    async fn case(channel: Channel) {
      let mut peer = RouterSocket::new();
      let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
      let protocol = protocol();
      let (driver, mut events) = ChannelDriver::connect(
        channel,
        &endpoint.to_string(),
        protocol.clone(),
        DriverConfig::default(),
      )
      .await
      .unwrap();

      driver.try_send(&envelope()).unwrap();

      let request =
        message_to_frames(peer.recv().await.unwrap(), &driver.config).unwrap();
      let mut request = protocol.decode(&request).unwrap();
      assert_eq!(request.buffers, envelope().buffers);
      assert_eq!(request.identities.len(), 1);

      request.identities.push(b"bar".to_vec());
      peer
        .send(frames_to_message(protocol.encode(&request).unwrap()).unwrap())
        .await
        .unwrap();

      let TransportEvent::Message(message) = event(&mut events).await else {
        panic!("expected message event");
      };

      assert_eq!(message.channel, channel);
      assert_eq!(message.envelope.buffers, envelope().buffers);
      assert_eq!(message.envelope.identities, [b"bar".to_vec()]);

      driver.shutdown().await.unwrap();
    }

    case(Channel::Control).await;
    case(Channel::Shell).await;
    case(Channel::Stdin).await;
  }

  #[tokio::test]
  async fn disconnect_is_a_typed_event() {
    let mut peer = RouterSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let (driver, mut events) = ChannelDriver::connect(
      Channel::Shell,
      &endpoint.to_string(),
      protocol(),
      DriverConfig::default(),
    )
    .await
    .unwrap();

    drop(peer);
    time::sleep(Duration::from_millis(20)).await;
    driver.try_send(&envelope()).unwrap();

    assert!(matches!(
      event(&mut events).await,
      TransportEvent::Error {
        channel: Channel::Shell,
        error: TransportError::Disconnected,
      }
    ));

    driver.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn heartbeat_echoes_raw_bytes() {
    let mut peer = RepSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let (driver, mut events) =
      HeartbeatDriver::connect(&endpoint.to_string(), DriverConfig::default())
        .await
        .unwrap();
    let ping = b"\0foo\xff".to_vec();

    driver.try_ping(ping.clone()).unwrap();

    let message = peer.recv().await.unwrap();
    assert_eq!(message.len(), 1);
    assert_eq!(message.get(0).unwrap().as_ref(), ping);
    peer.send(message).await.unwrap();

    assert!(matches!(
      event(&mut events).await,
      TransportEvent::Heartbeat(bytes) if bytes == ping
    ));

    driver.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn heartbeat_timeout_is_a_typed_event() {
    let mut peer = RepSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let config = DriverConfig {
      heartbeat_timeout: Duration::from_millis(20),
      ..DriverConfig::default()
    };
    let (driver, mut events) =
      HeartbeatDriver::connect(&endpoint.to_string(), config)
        .await
        .unwrap();

    driver.try_ping(b"foo".to_vec()).unwrap();
    peer.recv().await.unwrap();

    assert!(matches!(
      event(&mut events).await,
      TransportEvent::Error {
        channel: Channel::Heartbeat,
        error: TransportError::Timeout(_),
      }
    ));

    driver.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn iopub_receives_topics_and_buffers() {
    let mut peer = XPubSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let protocol = protocol();
    let (driver, mut events) = ChannelDriver::connect(
      Channel::Iopub,
      &endpoint.to_string(),
      protocol.clone(),
      DriverConfig::default(),
    )
    .await
    .unwrap();

    let subscription = peer.recv().await.unwrap();
    assert_eq!(subscription.get(0).unwrap().as_ref(), [1]);

    let mut message = envelope();
    message.identities.push(b"bar".to_vec());
    peer
      .send(frames_to_message(protocol.encode(&message).unwrap()).unwrap())
      .await
      .unwrap();

    let TransportEvent::Message(received) = event(&mut events).await else {
      panic!("expected message event");
    };

    assert_eq!(received.channel, Channel::Iopub);
    assert_eq!(received.envelope.identities, [b"bar".to_vec()]);
    assert_eq!(received.envelope.buffers, envelope().buffers);

    driver.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn malformed_and_bad_signature_messages_are_typed_events() {
    let mut peer = RouterSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let protocol = protocol();
    let (driver, mut events) = ChannelDriver::connect(
      Channel::Shell,
      &endpoint.to_string(),
      protocol.clone(),
      DriverConfig::default(),
    )
    .await
    .unwrap();

    driver.try_send(&envelope()).unwrap();
    let request = peer.recv().await.unwrap();
    let identity = request.get(0).unwrap().to_vec();
    let malformed = vec![identity.clone(), b"foo".to_vec()];
    peer
      .send(frames_to_message(malformed).unwrap())
      .await
      .unwrap();

    assert!(matches!(
      event(&mut events).await,
      TransportEvent::Error {
        channel: Channel::Shell,
        error: TransportError::Decode(WireError::MissingDelimiter),
      }
    ));

    let mut message = envelope();
    message.identities.push(identity);
    let mut frames = protocol.encode(&message).unwrap();
    let delimiter = frames.iter().position(|frame| frame == DELIMITER).unwrap();
    frames[delimiter + 1][0] ^= 1;
    peer.send(frames_to_message(frames).unwrap()).await.unwrap();

    assert!(matches!(
      event(&mut events).await,
      TransportEvent::Error {
        channel: Channel::Shell,
        error: TransportError::Signature(_),
      }
    ));

    driver.shutdown().await.unwrap();
  }

  #[test]
  fn outbound_queue_is_bounded() {
    let config = DriverConfig {
      queue_capacity: 1,
      ..DriverConfig::default()
    };
    let (cancellation, _cancelled) = watch::channel(false);
    let (commands, _command_receiver) = mpsc::channel(config.queue_capacity);
    let driver = ChannelDriver {
      cancellation,
      channel: Channel::Shell,
      commands,
      config,
      protocol: protocol(),
      task: None,
    };

    driver.try_send(&envelope()).unwrap();
    assert!(matches!(
      driver.try_send(&envelope()),
      Err(TransportError::QueueFull)
    ));
  }

  #[tokio::test]
  async fn oversized_inbound_message_is_a_typed_event() {
    let mut peer = RouterSocket::new();
    let endpoint = peer.bind("tcp://127.0.0.1:0").await.unwrap();
    let config = DriverConfig {
      max_message_frames: 8,
      ..DriverConfig::default()
    };
    let (driver, mut events) = ChannelDriver::connect(
      Channel::Shell,
      &endpoint.to_string(),
      protocol(),
      config,
    )
    .await
    .unwrap();

    driver.try_send(&envelope()).unwrap();
    let request = peer.recv().await.unwrap();
    let mut frames = vec![request.get(0).unwrap().to_vec()];
    frames.extend([const { Vec::new() }; 9]);
    peer.send(frames_to_message(frames).unwrap()).await.unwrap();

    assert!(matches!(
      event(&mut events).await,
      TransportEvent::Error {
        channel: Channel::Shell,
        error: TransportError::TooManyFrames {
          actual: 9,
          maximum: 8,
        },
      }
    ));

    driver.shutdown().await.unwrap();
  }

  #[test]
  fn rejects_invalid_limits_and_oversized_frames() {
    let invalid = DriverConfig {
      queue_capacity: 0,
      ..DriverConfig::default()
    };
    assert!(matches!(
      invalid.validate(),
      Err(TransportError::InvalidConfig)
    ));

    let config = DriverConfig {
      max_frame_bytes: 2,
      max_message_bytes: 4,
      ..DriverConfig::default()
    };
    assert!(matches!(
      validate_frame(b"foo", &config),
      Err(TransportError::FrameTooLarge {
        actual: 3,
        maximum: 2,
      })
    ));
    assert!(matches!(
      validate_lengths(
        [b"foo".as_slice(), b"bar"],
        &DriverConfig {
          max_frame_bytes: 3,
          max_message_bytes: 5,
          ..DriverConfig::default()
        }
      ),
      Err(TransportError::MessageTooLarge {
        actual: 6,
        maximum: 5,
      })
    ));
  }
}
