use futures::{future::BoxFuture, FutureExt};

use crate::core::logs::{consumer::LogConsumer, LogFrame};

/// A consumer that logs the output of container with the [`log`] crate.
///
/// By default, both standard out and standard error will both be emitted at INFO level.
#[derive(Debug)]
pub struct LoggingConsumer {
    stdout_level: log::Level,
    stderr_level: log::Level,
}

impl LoggingConsumer {
    /// Creates a new instance of the logging consumer.
    pub fn new() -> Self {
        Self {
            stdout_level: log::Level::Info,
            stderr_level: log::Level::Info,
        }
    }

    /// Sets the log level for standard out. By default, this is `INFO`.
    pub fn with_stdout_level(mut self, level: log::Level) -> Self {
        self.stdout_level = level;
        self
    }

    /// Sets the log level for standard error. By default, this is `INFO`.
    pub fn with_stderr_level(mut self, level: log::Level) -> Self {
        self.stderr_level = level;
        self
    }
}

impl Default for LoggingConsumer {
    fn default() -> Self {
        Self::new()
    }
}

impl LogConsumer for LoggingConsumer {
    fn accept<'a>(&'a self, record: &'a LogFrame) -> BoxFuture<'a, ()> {
        async move {
            match record {
                LogFrame::StdOut(bytes) => {
                    log::log!(self.stdout_level, "{}", String::from_utf8_lossy(bytes));
                }
                LogFrame::StdErr(bytes) => {
                    log::log!(self.stderr_level, "{}", String::from_utf8_lossy(bytes));
                }
            }
        }
        .boxed()
    }
}
