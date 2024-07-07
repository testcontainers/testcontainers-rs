use bytes::Bytes;

use crate::{
    core::{
        client::Client,
        error::WaitContainerError,
        logs::{LogSource, WaitingStreamWrapper},
        wait::WaitStrategy,
    },
    ContainerAsync, Image,
};

#[derive(Debug, Clone)]
pub struct LogWaitStrategy {
    source: LogSource,
    message: Bytes,
    times: usize,
}

impl LogWaitStrategy {
    /// Create a new [`LogWaitStrategy`] that waits for the given message to appear in the standard output logs.
    /// Shortcut for `LogWaitStrategy::new(LogSource::StdOut, message)`.
    pub fn stdout(message: impl AsRef<[u8]>) -> Self {
        Self::new(LogSource::StdOut, message)
    }

    /// Create a new [`LogWaitStrategy`] that waits for the given message to appear in the standard error logs.
    /// Shortcut for `LogWaitStrategy::new(LogSource::StdErr, message)`.
    pub fn stderr(message: impl AsRef<[u8]>) -> Self {
        Self::new(LogSource::StdErr, message)
    }

    /// Create a new `LogWaitStrategy` with the given log source and message.
    /// The message is expected to appear in the logs exactly once by default.
    pub fn new(source: LogSource, message: impl AsRef<[u8]>) -> Self {
        Self {
            source,
            message: Bytes::from(message.as_ref().to_vec()),
            times: 1,
        }
    }

    /// Set the number of times the message should appear in the logs.
    pub fn with_times(mut self, times: usize) -> Self {
        self.times = times;
        self
    }
}

impl WaitStrategy for LogWaitStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()> {
        let log_stream = match self.source {
            LogSource::StdOut => client.stdout_logs(container.id(), true),
            LogSource::StdErr => client.stderr_logs(container.id(), true),
        };

        WaitingStreamWrapper::new(log_stream)
            .wait_for_message(self.message, self.times)
            .await
            .map_err(WaitContainerError::from)?;

        Ok(())
    }
}
