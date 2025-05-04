use std::borrow::Cow;

use futures::{future::BoxFuture, FutureExt};

use crate::core::logs::{consumer::LogConsumer, LogFrame};

/// A consumer that logs the output of container with the [`log`] crate.
///
/// By default, both standard out and standard error will both be emitted at INFO level.
#[derive(Debug)]
pub struct LoggingConsumer {
    stdout_level: log::Level,
    stderr_level: log::Level,
    prefix: Option<String>,
}

impl LoggingConsumer {
    /// Creates a new instance of the logging consumer.
    pub fn new() -> Self {
        Self {
            stdout_level: log::Level::Info,
            stderr_level: log::Level::Info,
            prefix: None,
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

    /// Sets a prefix to be added to each log message (space will be added between prefix and message).
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    fn format_message<'a>(&self, message: &'a str) -> Cow<'a, str> {
        // Remove trailing newlines
        let message = message.trim_end_matches(['\n', '\r']);

        if let Some(prefix) = &self.prefix {
            Cow::Owned(format!("{prefix} {message}"))
        } else {
            Cow::Borrowed(message)
        }
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
                    log::log!(
                        self.stdout_level,
                        "{}",
                        self.format_message(&String::from_utf8_lossy(bytes))
                    );
                }
                LogFrame::StdErr(bytes) => {
                    log::log!(
                        self.stderr_level,
                        "{}",
                        self.format_message(&String::from_utf8_lossy(bytes))
                    );
                }
            }
        }
        .boxed()
    }
}
