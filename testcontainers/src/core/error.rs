use std::error::Error;

pub use crate::core::{client::ClientError, env::ConfigurationError, ContainerPort};
use crate::core::{logs::WaitLogError, wait::http_strategy::HttpWaitError};

pub type Result<T> = std::result::Result<T, TestcontainersError>;

/// Enum to represent various types of errors that can occur in Testcontainers
#[derive(Debug, thiserror::Error)]
pub enum TestcontainersError {
    /// Represents an error that occurred in the client of Docker API.
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("container is not ready: {0}")]
    WaitContainer(#[from] WaitContainerError),
    /// Represents an error when a container does not expose a specified port
    #[error("container '{id}' does not expose port {port}")]
    PortNotExposed { id: String, port: ContainerPort },
    /// Represents an error when a container is missing some information
    #[error(transparent)]
    MissingInfo(#[from] ContainerMissingInfo),
    /// Represents an error when an exec operation fails
    #[error("exec operation failed: {0}")]
    Exec(#[from] ExecError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Represents any other error that does not fit into the above categories
    #[error("other error: {0}")]
    Other(Box<dyn Error + Sync + Send>),
}

#[derive(Debug, thiserror::Error)]
#[error("container '{id}' does not have: {path}")]
pub struct ContainerMissingInfo {
    /// Container ID
    id: String,
    /// Path to the missing information (e.g `NetworkSettings.Networks`).
    path: String,
}

/// Error type for exec operation.
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("exec process exited with code {actual}, expected {expected}")]
    ExitCodeMismatch { expected: i64, actual: i64 },
    #[error("failed to wait for exec log: {0}")]
    WaitLog(#[from] WaitLogError),
}

/// Error type for waiting for container readiness based on [`crate::core::WaitFor`] conditions.
#[derive(Debug, thiserror::Error)]
pub enum WaitContainerError {
    #[error("failed to wait for container log: {0}")]
    WaitLog(#[from] WaitLogError),
    #[error("container state is unavailable")]
    StateUnavailable,
    #[error("container is not ready: {0}")]
    HttpWait(#[from] HttpWaitError),
    #[error("healthcheck is not configured for container: {0}")]
    HealthCheckNotConfigured(String),
    #[error("container is unhealthy")]
    Unhealthy,
    #[error("container startup timeout")]
    StartupTimeout,
    #[error("container exited with unexpected code: expected {expected}, actual {actual:?}")]
    UnexpectedExitCode { expected: i64, actual: Option<i64> },
}

impl TestcontainersError {
    /// Creates a new `TestcontainersError` from an arbitrary error payload.
    ///
    /// It's preferably to use the more specific error constructors if possible.
    /// But this method is useful when you need to:
    /// - wrap an error that doesn't fit into the other categories
    /// - avoid introducing a new kind of error in order to keep the error handling simple
    /// - create a custom error from client code.
    pub fn other<E>(error: E) -> Self
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        Self::Other(error.into())
    }
}

impl ContainerMissingInfo {
    pub(crate) fn new(id: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
        }
    }
}
