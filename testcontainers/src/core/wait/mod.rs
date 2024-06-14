use std::{env::var, fmt::Debug, time::Duration};

use bytes::Bytes;
use http_strategy::HttpWaitStrategy;

use crate::Image;

pub(crate) mod cmd_wait;
pub(crate) mod http_strategy;

pub(crate) trait WaitStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        container: &crate::ContainerAsync<I>,
    ) -> crate::core::error::Result<()>;
}

/// Represents a condition that needs to be met before a container is considered ready.
#[derive(Debug, Clone)]
pub enum WaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a message on the stdout stream of the container's logs.
    StdOutMessage { message: Bytes },
    /// Wait for a message on the stderr stream of the container's logs.
    StdErrMessage { message: Bytes },
    /// Wait for a certain amount of time.
    Duration { length: Duration },
    /// Wait for the container's status to become `healthy`.
    Healthcheck,
    /// Wait for a certain HTTP response.
    Http(HttpWaitStrategy),
}

impl WaitFor {
    /// Wait for the message to appear on the container's stdout.
    pub fn message_on_stdout(message: impl AsRef<[u8]>) -> WaitFor {
        WaitFor::StdOutMessage {
            message: Bytes::from(message.as_ref().to_vec()),
        }
    }

    /// Wait for the message to appear on the container's stderr.
    pub fn message_on_stderr(message: impl AsRef<[u8]>) -> WaitFor {
        WaitFor::StdErrMessage {
            message: Bytes::from(message.as_ref().to_vec()),
        }
    }

    /// Wait for the container to become healthy.
    pub fn healthcheck() -> WaitFor {
        WaitFor::Healthcheck
    }

    /// Wait for a certain HTTP response.
    pub fn http(http_strategy: HttpWaitStrategy) -> WaitFor {
        WaitFor::Http(http_strategy)
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
