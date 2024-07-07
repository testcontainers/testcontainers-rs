use std::time::Duration;

use crate::{
    core::{client::Client, error::WaitContainerError, wait::WaitStrategy},
    ContainerAsync, Image,
};

#[derive(Debug, Clone)]
pub struct ExitWaitStrategy {
    expected_code: Option<i64>,
    poll_interval: Duration,
}

impl ExitWaitStrategy {
    /// Create a new `ExitWaitStrategy` with default settings.
    pub fn new() -> Self {
        Self {
            expected_code: None,
            poll_interval: Duration::from_millis(100),
        }
    }

    /// Set the poll interval for checking the container's status.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Set the expected exit code of the container.
    pub fn with_exit_code(mut self, expected_code: i64) -> Self {
        self.expected_code = Some(expected_code);
        self
    }
}

impl WaitStrategy for ExitWaitStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()> {
        loop {
            let container_state = client
                .inspect(container.id())
                .await?
                .state
                .ok_or(WaitContainerError::StateUnavailable)?;

            let is_running = container_state.running.unwrap_or_default();

            if is_running {
                tokio::time::sleep(self.poll_interval).await;
                continue;
            }

            if let Some(expected_code) = self.expected_code {
                let exit_code = container_state.exit_code;
                if exit_code != Some(expected_code) {
                    return Err(WaitContainerError::UnexpectedExitCode {
                        expected: expected_code,
                        actual: exit_code,
                    }
                    .into());
                }
            }
            break;
        }
        Ok(())
    }
}

impl Default for ExitWaitStrategy {
    fn default() -> Self {
        Self::new()
    }
}
