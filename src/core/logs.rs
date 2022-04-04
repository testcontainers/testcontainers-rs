#[cfg(feature = "experimental")]
use futures::{stream::BoxStream, StreamExt};
use std::{
    fmt, io,
    io::{BufRead, BufReader, Read},
};

#[cfg(feature = "experimental")]
pub(crate) struct LogStreamAsync<'d> {
    inner: BoxStream<'d, Result<String, std::io::Error>>,
}

#[cfg(feature = "experimental")]
impl<'d> fmt::Debug for LogStreamAsync<'d> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamAsync").finish()
    }
}

#[cfg(feature = "experimental")]
impl<'d> LogStreamAsync<'d> {
    pub fn new(stream: BoxStream<'d, Result<String, std::io::Error>>) -> Self {
        Self { inner: stream }
    }

    pub async fn wait_for_message(mut self, message: &str) -> Result<(), WaitError> {
        let mut lines = vec![];

        while let Some(line) = self.inner.next().await.transpose()? {
            if handle_line(line, message, &mut lines) {
                return Ok(());
            }
        }

        Err(end_of_stream(lines))
    }
}

pub(crate) struct LogStream {
    inner: Box<dyn Read>,
}

impl fmt::Debug for LogStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStream").finish()
    }
}

impl LogStream {
    pub fn new(stream: impl Read + 'static) -> Self {
        Self {
            inner: Box::new(stream),
        }
    }

    pub fn wait_for_message(self, message: &str) -> Result<(), WaitError> {
        let logs = BufReader::new(self.inner);
        let mut lines = vec![];

        for line in logs.lines() {
            if handle_line(line?, message, &mut lines) {
                return Ok(());
            }
        }

        Err(end_of_stream(lines))
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

fn end_of_stream(lines: Vec<String>) -> WaitError {
    log::error!(
        "Failed to find message in stream after comparing {} lines.",
        lines.len()
    );

    WaitError::EndOfStream(lines)
}

/// Defines error cases when waiting for a message in a stream.
#[derive(Debug)]
pub enum WaitError {
    /// Indicates the stream ended before finding the log line you were looking for.
    /// Contains all the lines that were read for debugging purposes.
    EndOfStream(Vec<String>),
    Io(io::Error),
}

impl From<io::Error> for WaitError {
    fn from(e: io::Error) -> Self {
        WaitError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_logs_when_line_contains_message_should_find_it() {
        let log_stream = LogStream::new(
            r"
            Message one
            Message two
            Message three
        "
            .as_bytes(),
        );

        let result = log_stream.wait_for_message("Message three");

        assert!(result.is_ok())
    }
}
