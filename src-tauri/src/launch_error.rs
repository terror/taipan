use super::*;

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
