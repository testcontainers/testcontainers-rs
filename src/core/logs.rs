use crate::WaitError;
use futures::{stream::BoxStream, StreamExt};
use std::fmt;

pub(crate) struct LogStreamAsync<'d> {
    inner: BoxStream<'d, Result<String, std::io::Error>>,
}

impl<'d> fmt::Debug for LogStreamAsync<'d> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamAsync").finish()
    }
}

impl<'d> LogStreamAsync<'d> {
    pub fn new(stream: BoxStream<'d, Result<String, std::io::Error>>) -> Self {
        Self { inner: stream }
    }

    pub async fn wait_for_message(mut self, message: &str) -> Result<(), WaitError> {
        let mut lines = vec![];

        while let Some(line) = self.inner.next().await.transpose()? {
            if line.contains(message) {
                return Ok(());
            }

            lines.push(line);
        }

        Err(WaitError::EndOfStream(lines))
    }
}
