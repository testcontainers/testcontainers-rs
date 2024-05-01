use std::{fmt, io};

use futures::{stream::BoxStream, StreamExt};

pub(crate) struct LogStreamAsync<'d> {
    inner: BoxStream<'d, Result<String, io::Error>>,
}

impl<'d> fmt::Debug for LogStreamAsync<'d> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamAsync").finish()
    }
}

impl<'d> LogStreamAsync<'d> {
    pub fn new(stream: BoxStream<'d, Result<String, io::Error>>) -> Self {
        Self { inner: stream }
    }

    pub async fn wait_for_message(mut self, message: &str) -> Result<(), WaitError> {
        let mut lines = vec![];

        while let Some(line) = self.inner.next().await.transpose()? {
            if handle_line(line, message, &mut lines) {
                return Ok(());
            }
        }

        Err(end_of_stream(message, lines))
    }
}

fn handle_line(line: String, message: &str, lines: &mut Vec<String>) -> bool {
    if line.contains(message) {
        log::info!("Found message after comparing {} lines", lines.len());

        return true;
    }

    lines.push(line);

    false
}

fn end_of_stream(expected_msg: &str, lines: Vec<String>) -> WaitError {
    log::error!(
        "Failed to find message '{expected_msg}' in stream after comparing {} lines.",
        lines.len()
    );

    WaitError::EndOfStream(lines)
}

/// Defines error cases when waiting for a message in a stream.
#[derive(Debug)]
pub enum WaitError {
    /// Indicates the stream ended before finding the log line you were looking for.
    /// Contains all the lines that were read for debugging purposes.
    EndOfStream(#[allow(dead_code)] Vec<String>), // todo: tuple is used by Debug impl, remove once nightly clippy is fixed
    Io(#[allow(dead_code)] io::Error),
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
        let log_stream = LogStreamAsync::new(Box::pin(futures::stream::iter([Ok(r"
            Message one
            Message two
            Message three
        "
        .to_string())])));

        let result = log_stream.wait_for_message("Message three").await;

        assert!(result.is_ok())
    }
}
