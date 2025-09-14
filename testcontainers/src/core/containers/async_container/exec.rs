use std::{fmt, io, pin::Pin, sync::Arc};

use bytes::Bytes;
use futures::stream::BoxStream;
use tokio::io::{AsyncBufRead, AsyncReadExt};

use crate::core::{client::Client, error::Result};

/// Represents the result of an executed command in a container.
pub struct ExecResult {
    pub(super) client: Arc<Client>,
    pub(crate) id: String,
    pub(super) stdout: BoxStream<'static, std::result::Result<Bytes, io::Error>>,
    pub(super) stderr: BoxStream<'static, std::result::Result<Bytes, io::Error>>,
}

impl ExecResult {
    /// Returns the exit code of the executed command.
    /// If the command has not yet exited, this will return `None`.
    pub async fn exit_code(&self) -> Result<Option<i64>> {
        let res = self.client.inspect_exec(&self.id).await?;
        Ok(res.exit_code)
    }

    /// Returns an asynchronous reader for stdout. It follows log stream until the command exits.
    pub fn stdout<'b>(&'b mut self) -> Pin<Box<dyn AsyncBufRead + Send + 'b>> {
        Box::pin(tokio_util::io::StreamReader::new(&mut self.stdout))
    }

    /// Returns an asynchronous reader for stderr. It follows log stream until the command exits.
    pub fn stderr<'b>(&'b mut self) -> Pin<Box<dyn AsyncBufRead + Send + 'b>> {
        Box::pin(tokio_util::io::StreamReader::new(&mut self.stderr))
    }

    /// Returns stdout as a vector of bytes.
    /// Keep in mind that this will block until the command exits.
    ///
    /// If you want to read stdout in asynchronous manner, use [`ExecResult::stdout`] instead.
    pub async fn stdout_to_vec(&mut self) -> Result<Vec<u8>> {
        let mut stdout = Vec::new();
        self.stdout().read_to_end(&mut stdout).await?;
        Ok(stdout)
    }

    /// Returns stderr as a vector of bytes.
    /// Keep in mind that this will block until the command exits.
    ///
    /// If you want to read stderr in asynchronous manner, use [`ExecResult::stderr`] instead.
    pub async fn stderr_to_vec(&mut self) -> Result<Vec<u8>> {
        let mut stderr = Vec::new();
        self.stderr().read_to_end(&mut stderr).await?;
        Ok(stderr)
    }
}

impl fmt::Debug for ExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult").field("id", &self.id).finish()
    }
}
