use std::{env::var, fmt::Debug, time::Duration};

pub use exit_strategy::ExitWaitStrategy;
pub use health_strategy::HealthWaitStrategy;
pub use http_strategy::HttpWaitStrategy;
pub use log_strategy::LogWaitStrategy;

use crate::{
    core::{client::Client, logs::LogSource},
    ContainerAsync, Image,
};

pub(crate) mod cmd_wait;
pub(crate) mod exit_strategy;
pub(crate) mod health_strategy;
pub(crate) mod http_strategy;
pub(crate) mod log_strategy;

pub(crate) trait WaitStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()>;
}

/// Represents a condition that needs to be met before a container is considered ready.
#[derive(Debug, Clone)]
pub enum WaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a certain message to appear in the container's logs.
    Log(LogWaitStrategy),
    /// Wait for a certain amount of time.
    Duration { length: Duration },
    /// Wait for the container's status to become `healthy`.
    Healthcheck(HealthWaitStrategy),
    /// Wait for a certain HTTP response.
    Http(HttpWaitStrategy),
    /// Wait for the container to exit.
    Exit(ExitWaitStrategy),
}

impl WaitFor {
    /// Wait for the message to appear on the container's stdout.
    pub fn message_on_stdout(message: impl AsRef<[u8]>) -> WaitFor {
        Self::log(LogWaitStrategy::new(LogSource::StdOut, message))
    }

    /// Wait for the message to appear on the container's stderr.
    pub fn message_on_stderr(message: impl AsRef<[u8]>) -> WaitFor {
        Self::log(LogWaitStrategy::new(LogSource::StdErr, message))
    }

    /// Wait for the message to appear on the container's stdout.
    pub fn log(log_strategy: LogWaitStrategy) -> WaitFor {
        WaitFor::Log(log_strategy)
    }

    /// Wait for the container to become healthy.
    ///
    /// If you need to customize polling interval, use [`HealthWaitStrategy::with_poll_interval`]
    /// and create the strategy [`WaitFor::Healthcheck`] manually.
    pub fn healthcheck() -> WaitFor {
        WaitFor::Healthcheck(HealthWaitStrategy::default())
    }

    /// Wait for a certain HTTP response.
    pub fn http(http_strategy: HttpWaitStrategy) -> WaitFor {
        WaitFor::Http(http_strategy)
    }

    /// Wait for the container to exit.
    pub fn exit(exit_strategy: ExitWaitStrategy) -> WaitFor {
        WaitFor::Exit(exit_strategy)
    }

    /// Wait for a certain amount of seconds.
    ///
    /// Generally, it's not recommended to use this method, as it's better to wait for a specific condition to be met.
    pub fn seconds(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_secs(length),
        }
    }

    /// Wait for a certain amount of millis.
    ///
    /// Generally, it's not recommended to use this method, as it's better to wait for a specific condition to be met.
    pub fn millis(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_millis(length),
        }
    }

    /// Wait for a certain amount of millis specified in the environment variable.
    ///
    /// Generally, it's not recommended to use this method, as it's better to wait for a specific condition to be met.
    pub fn millis_in_env_var(name: &'static str) -> WaitFor {
        let additional_sleep_period = var(name).map(|value| value.parse());

        (|| {
            let length = additional_sleep_period.ok()?.ok()?;

            Some(WaitFor::Duration {
                length: Duration::from_millis(length),
            })
        })()
        .unwrap_or(WaitFor::Nothing)
    }
}

impl From<HttpWaitStrategy> for WaitFor {
    fn from(value: HttpWaitStrategy) -> Self {
        Self::Http(value)
    }
}

impl WaitStrategy for WaitFor {
    async fn wait_until_ready<I: Image>(
        self,
        client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()> {
        match self {
            WaitFor::Log(strategy) => strategy.wait_until_ready(client, container).await?,
            WaitFor::Duration { length } => {
                tokio::time::sleep(length).await;
            }
            WaitFor::Healthcheck(strategy) => {
                strategy.wait_until_ready(client, container).await?;
            }
            WaitFor::Http(strategy) => {
                strategy.wait_until_ready(client, container).await?;
            }
            WaitFor::Exit(strategy) => {
                strategy.wait_until_ready(client, container).await?;
            }
            WaitFor::Nothing => {}
        }
        Ok(())
    }
}
