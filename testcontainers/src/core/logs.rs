use std::{borrow::Cow, fmt, io};

use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use memchr::memmem::Finder;

pub mod consumer;
pub(crate) mod stream;

#[derive(Debug, Clone)]
pub enum LogFrame {
    StdOut(Bytes),
    StdErr(Bytes),
}

/// Defines error cases when waiting for a message in a stream.
#[derive(Debug, thiserror::Error)]
pub enum WaitLogError {
    /// Indicates the stream ended before finding the log line you were looking for.
    /// Contains all the lines that were read for debugging purposes.
    #[error("End of stream reached before finding message: {:?}", display_bytes(.0))]
    EndOfStream(Vec<Bytes>),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Copy, Clone, Debug, parse_display::Display)]
#[display(style = "lowercase")]
pub enum LogSource {
    StdOut,
    StdErr,
}

impl LogSource {
    pub(super) fn is_stdout(self) -> bool {
        matches!(self, Self::StdOut)
    }

    pub(super) fn is_stderr(self) -> bool {
        matches!(self, Self::StdErr)
    }
}

impl LogFrame {
    pub fn source(&self) -> LogSource {
        match self {
            LogFrame::StdOut(_) => LogSource::StdOut,
            LogFrame::StdErr(_) => LogSource::StdErr,
        }
    }

    pub fn bytes(&self) -> &Bytes {
        match self {
            LogFrame::StdOut(bytes) => bytes,
            LogFrame::StdErr(bytes) => bytes,
        }
    }
}

// TODO: extract caching functionality to a separate wrapper
pub(crate) struct WaitingStreamWrapper {
    inner: BoxStream<'static, Result<Bytes, io::Error>>,
    cache: Vec<Result<Bytes, io::Error>>,
    enable_cache: bool,
}

impl WaitingStreamWrapper {
    pub fn new(stream: BoxStream<'static, Result<Bytes, io::Error>>) -> Self {
        Self {
            inner: stream,
            cache: vec![],
            enable_cache: false,
        }
    }

    pub fn enable_cache(mut self) -> Self {
        self.enable_cache = true;
        self
    }

    pub(crate) async fn wait_for_message(
        &mut self,
        message: impl AsRef<[u8]>,
        times: usize,
    ) -> Result<(), WaitLogError> {
        let msg_finder = Finder::new(message.as_ref());
        let mut messages = Vec::new();
        let mut found_times: usize = 0;
        while let Some(message) = self.inner.next().await.transpose()? {
            messages.push(message.clone());
            if self.enable_cache {
                self.cache.push(Ok(message.clone()));
            }

            let find_iter = msg_finder.find_iter(message.as_ref());
            for _ in find_iter {
                found_times += 1; // can't overflow, because of check below
                if found_times == times {
                    log::debug!(
                        "Message found {times} times after comparing {} lines",
                        messages.len()
                    );
                    return Ok(());
                }
            }
        }

        log::warn!(
            "Failed to find message '{}' {times} times after comparing {} lines.",
            String::from_utf8_lossy(message.as_ref()),
            messages.len()
        );
        Err(WaitLogError::EndOfStream(messages))
    }

    pub(crate) fn into_inner(self) -> BoxStream<'static, Result<Bytes, io::Error>> {
        futures::stream::iter(self.cache).chain(self.inner).boxed()
    }
}

fn display_bytes(bytes: &[Bytes]) -> Vec<Cow<'_, str>> {
    bytes
        .iter()
        .map(|m| String::from_utf8_lossy(m.as_ref()))
        .collect::<Vec<_>>()
}

impl fmt::Debug for WaitingStreamWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamAsync").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn given_logs_when_line_contains_message_should_find_it() {
        let _ = pretty_env_logger::try_init();
        let log_stream = || {
            WaitingStreamWrapper::new(Box::pin(futures::stream::iter([
                Ok(r"
            Message one
            Message two
            Message three
            Message three
        "
                .into()),
                Ok("Message three".into()),
            ])))
        };

        let result = log_stream().wait_for_message("Message one", 1).await;
        assert!(result.is_ok());

        let result = log_stream().wait_for_message("Message two", 2).await;
        assert!(result.is_err());

        let result = log_stream().wait_for_message("Message three", 1).await;
        assert!(result.is_ok());

        let result = log_stream().wait_for_message("Message three", 3).await;
        assert!(result.is_ok());
    }
}
