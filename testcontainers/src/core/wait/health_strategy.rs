use std::time::Duration;

use bollard::models::HealthStatusEnum::*;

use crate::{
    core::{client::Client, error::WaitContainerError, wait::WaitStrategy},
    ContainerAsync, Image,
};

#[derive(Debug, Clone)]
pub struct HealthWaitStrategy {
    poll_interval: Duration,
}

impl HealthWaitStrategy {
    /// Create a new `HealthWaitStrategy` with default settings.
    pub fn new() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
        }
    }

    /// Set the poll interval for checking the container's health status.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }
}

impl WaitStrategy for HealthWaitStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()> {
        loop {
            let health_status = client
                .inspect(container.id())
                .await?
                .state
                .ok_or(WaitContainerError::StateUnavailable)?
                .health
                .and_then(|health| health.status);

            match health_status {
                Some(HEALTHY) => break,
                None | Some(EMPTY) | Some(NONE) => Err(
                    WaitContainerError::HealthCheckNotConfigured(container.id().to_string()),
                )?,
                Some(UNHEALTHY) => Err(WaitContainerError::Unhealthy)?,
                Some(STARTING) => {
                    tokio::time::sleep(self.poll_interval).await;
                }
            }
        }
        Ok(())
    }
}

impl Default for HealthWaitStrategy {
    fn default() -> Self {
        Self::new()
    }
}
