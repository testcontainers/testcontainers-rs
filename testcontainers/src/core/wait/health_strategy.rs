use std::time::Duration;

use bollard::models::HealthStatusEnum::*;

use crate::{
    core::{client::Client, error::WaitContainerError, wait::WaitStrategy},
    ContainerAsync, Image,
};

pub(crate) struct HealthWaitStrategy;

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
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        Ok(())
    }
}
