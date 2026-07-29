use super::*;

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
  Task(#[source] JoinError),
}
