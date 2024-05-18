use crate::core::logs::WaitLogError;

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("failed to start exec process: {0}")]
    Init(#[from] bollard::errors::Error),
    #[error("exec process exited with code {actual}, expected {expected}")]
    ExitCodeMismatch { expected: i64, actual: i64 },
    #[error("failed to wait for exec log: {0}")]
    WaitLog(#[from] WaitLogError),
    #[error("container's wait conditions are not met: {0}")]
    WaitContainer(#[from] WaitContainerError),
}

#[derive(Debug, thiserror::Error)]
pub enum WaitContainerError {
    #[error("failed to wait for container log: {0}")]
    WaitLog(#[from] WaitLogError),
    #[error("failed to inspect container: {0}")]
    Inspect(bollard::errors::Error),
    #[error("container state is unavailable")]
    StateUnavailable,
    #[error("healthcheck is not configured for container: {0}")]
    HealthCheckNotConfigured(String),
    #[error("container is unhealthy")]
    Unhealthy,
}
