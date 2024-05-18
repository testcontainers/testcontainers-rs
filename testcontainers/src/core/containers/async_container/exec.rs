use std::{fmt, io, pin::Pin, sync::Arc};

use bytes::Bytes;
use futures::stream::BoxStream;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::core::client::Client;

/// Represents the result of an executed command in a container.
pub struct ExecResult<'a> {
    pub(super) client: Arc<Client>,
    pub(crate) id: String,
    pub(super) stdout: BoxStream<'a, Result<Bytes, io::Error>>,
    pub(super) stderr: BoxStream<'a, Result<Bytes, io::Error>>,
}

impl<'a> ExecResult<'a> {
    /// Returns the exit code of the executed command.
    /// If the command has not yet exited, this will return `None`.
    pub async fn exit_code(&self) -> Result<Option<i64>, bollard::errors::Error> {
        self.client
            .inspect_exec(&self.id)
            .await
            .map(|exec| exec.exit_code)
    }

    /// Returns stdout as a vector of bytes.
    /// If you want to read stdout in asynchronous manner, use `stdout_reader` instead.
    pub async fn stdout(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut stdout = Vec::new();
        self.stdout_reader().read_to_end(&mut stdout).await?;
        Ok(stdout)
    }

    /// Returns stderr as a vector of bytes.
    /// If you want to read stderr in asynchronous manner, use `stderr_reader` instead.
    pub async fn stderr(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut stderr = Vec::new();
        self.stderr_reader().read_to_end(&mut stderr).await?;
        Ok(stderr)
    }

    /// Returns an asynchronous reader for stdout.
    pub fn stdout_reader<'b>(&'b mut self) -> Pin<Box<dyn AsyncRead + 'b>> {
        Box::pin(tokio_util::io::StreamReader::new(&mut self.stdout))
    }

    /// Returns an asynchronous reader for stderr.
    pub fn stderr_reader<'b>(&'b mut self) -> Pin<Box<dyn AsyncRead + 'b>> {
        Box::pin(tokio_util::io::StreamReader::new(&mut self.stderr))
    }
}

impl fmt::Debug for ExecResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult").field("id", &self.id).finish()
    }
}
