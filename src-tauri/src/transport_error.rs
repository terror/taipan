use super::*;

#[derive(Debug, Error)]
pub enum TransportError {
  #[error("failed to connect ZeroMQ socket: {0}")]
  Connect(#[source] ZmqError),
  #[error("failed to decode Jupyter message: {0}")]
  Decode(#[source] WireError),
  #[error("ZeroMQ peer disconnected")]
  Disconnected,
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
