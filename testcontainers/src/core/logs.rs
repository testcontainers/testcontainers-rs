use std::{fmt, io};

use bytes::Bytes;
use futures::{stream::BoxStream, Stream, StreamExt};

/// Defines error cases when waiting for a message in a stream.
#[derive(Debug, thiserror::Error)]
pub enum WaitError {
    /// Indicates the stream ended before finding the log line you were looking for.
    /// Contains all the lines that were read for debugging purposes.
    #[error("End of stream reached: {0:?}")]
    EndOfStream(Vec<String>),
    Io(io::Error),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum LogSource {
    StdOut,
    StdErr,
}

pub(crate) type LogEntry = (LogSource, Bytes);

pub(crate) struct LogStreamAsync<'d> {
    inner: BoxStream<'d, Result<LogEntry, io::Error>>,
    stdout_cache: Vec<String>,
    stderr_cache: Vec<String>,
}

impl<'d> fmt::Debug for LogStreamAsync<'d> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamAsync").finish()
    }
}

impl<'d> LogStreamAsync<'d> {
    pub fn new(stream: BoxStream<'d, Result<LogEntry, io::Error>>) -> Self {
        Self {
            inner: stream,
            stdout_cache: vec![],
            stderr_cache: vec![],
        }
    }

    pub async fn wait_for_message(
        &mut self,
        message: &str,
        filter: Option<LogSource>,
    ) -> Result<(), WaitError> {
        while let Some((source, line)) = self.poll_line().await? {
            let match_found = match (source, filter) {
                (LogSource::StdOut, Some(LogSource::StdOut)) => {
                    check_line(line, message, self.stdout_cache.len())
                }
                (LogSource::StdErr, Some(LogSource::StdErr)) => {
                    check_line(line, message, self.stderr_cache.len())
                }
                (_, None) => check_line(
                    line,
                    message,
                    self.stdout_cache
                        .len()
                        .saturating_add(self.stderr_cache.len()),
                ),
                _ => false,
            };
            if match_found {
                return Ok(());
            }
        }

        log::error!(
            "Failed to find message '{message}' in stream after comparing {} lines.",
            1
        );
        Err(WaitError::EndOfStream {
            stdout: &self.stdout_cache,
            stderr: &self.stderr_cache,
        })
    }

    async fn poll_line(&mut self) -> Result<Option<(LogSource, &str)>, WaitError<'_>> {
        let line = if let Some(entry) = self.inner.next().await.transpose()? {
            match entry {
                (LogSource::StdOut, line) => {
                    self.stdout_cache.push(bytes_to_string(line)?);
                    self.stdout_cache
                        .last()
                        .map(|s| (LogSource::StdOut, s.as_str()))
                }
                (LogSource::StdErr, line) => {
                    self.stderr_cache.push(bytes_to_string(line)?);
                    self.stderr_cache
                        .last()
                        .map(|s| (LogSource::StdErr, s.as_str()))
                }
            }
        } else {
            None
        };

        Ok(line)
    }
}

fn check_line(line: &str, message: &str, handled_lines: usize) -> bool {
    if line.contains(message) {
        log::info!("Found message after comparing {handled_lines} lines");

        return true;
    }

    false
}

fn bytes_to_string(bytes: Bytes) -> Result<String, WaitError<'static>> {
    std::str::from_utf8(bytes.as_ref())
        .map_err(|e| WaitError::Io(io::Error::new(io::ErrorKind::Other, e)))
        .map(String::from)
}

impl From<io::Error> for WaitError {
    fn from(e: io::Error) -> Self {
        WaitError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn given_logs_when_line_contains_message_should_find_it() {
        let mut log_stream = LogStreamAsync::new(Box::pin(futures::stream::iter([Ok((
            LogSource::StdOut,
            r"
            Message one
            Message two
            Message three
        "
            .to_string(),
        ))])));

        let result = log_stream.wait_for_message("Message three", None).await;

        assert!(result.is_ok())
    }
}
