use crate::core::{client::ClientError, error::TestcontainersError};

/// Errors that can occur when working with Docker Compose
#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    #[error("Service '{0}' not found in compose stack")]
    ServiceNotFound(String),
    #[error("Testcontainers error: {0}")]
    Testcontainers(#[from] TestcontainersError),
}

pub type Result<T> = std::result::Result<T, ComposeError>;

impl From<ClientError> for ComposeError {
    fn from(err: ClientError) -> Self {
        ComposeError::Testcontainers(TestcontainersError::from(err))
    }
}
