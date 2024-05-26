use std::{borrow::Cow, fmt, io};

use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use memchr::memmem::Finder;

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
pub(crate) enum LogSource {
    StdOut,
    StdErr,
}

impl LogSource {
    pub(super) fn is_stdout(&self) -> bool {
        matches!(self, Self::StdOut)
    }

    pub(super) fn is_stderr(&self) -> bool {
        matches!(self, Self::StdErr)
    }
}

pub(crate) struct LogStreamAsync {
    inner: BoxStream<'static, Result<Bytes, io::Error>>,
    cache: Vec<Result<Bytes, io::Error>>,
    enable_cache: bool,
}

impl LogStreamAsync {
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
    ) -> Result<(), WaitLogError> {
        let msg_finder = Finder::new(message.as_ref());
        let mut messages = Vec::new();
        while let Some(message) = self.inner.next().await.transpose()? {
            messages.push(message.clone());
            if self.enable_cache {
                self.cache.push(Ok(message.clone()));
            }
            let match_found = msg_finder.find(message.as_ref()).is_some();
            if match_found {
                log::debug!("Found message after comparing {} lines", messages.len());
                return Ok(());
            }
        }

        log::warn!(
            "Failed to find message '{}' after comparing {} lines.",
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

impl fmt::Debug for LogStreamAsync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamAsync").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn given_logs_when_line_contains_message_should_find_it() {
        let mut log_stream = LogStreamAsync::new(Box::pin(futures::stream::iter([Ok(r"
            Message one
            Message two
            Message three
        "
        .into())])));

        let result = log_stream.wait_for_message("Message three").await;

        assert!(result.is_ok())
    }
}
