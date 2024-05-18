use std::{fmt, io, io::Read};

use tokio_util::io::SyncIoBridge;

/// Represents the result of an executed command in a container.
pub struct ExecResult<'a> {
    pub(super) inner: crate::core::async_container::exec::ExecResult<'a>,
    pub(super) runtime: &'a tokio::runtime::Runtime,
}

impl<'a> ExecResult<'a> {
    /// Returns the exit code of the executed command.
    /// If the command has not yet exited, this will return `None`.
    pub fn exit_code(&self) -> Result<Option<i64>, bollard::errors::Error> {
        self.runtime.block_on(self.inner.exit_code())
    }

    /// Returns stdout as a vector of bytes.
    /// If you want to read stdout in asynchronous manner, use `stdout_reader` instead.
    pub fn stdout(&mut self) -> Result<Vec<u8>, io::Error> {
        self.runtime.block_on(self.inner.stdout())
    }

    /// Returns stderr as a vector of bytes.
    /// If you want to read stderr in asynchronous manner, use `stderr_reader` instead.
    pub fn stderr(&mut self) -> Result<Vec<u8>, io::Error> {
        self.runtime.block_on(self.inner.stderr())
    }

    /// Returns an asynchronous reader for stdout.
    pub fn stdout_reader<'b>(&'b mut self) -> Box<dyn Read + 'b> {
        let reader = self.inner.stdout_reader();
        Box::new(SyncIoBridge::new_with_handle(
            reader,
            self.runtime.handle().clone(),
        ))
    }

    /// Returns an asynchronous reader for stderr.
    pub fn stderr_reader<'b>(&'b mut self) -> Box<dyn Read + 'b> {
        let reader = self.inner.stderr_reader();
        Box::new(SyncIoBridge::new_with_handle(
            reader,
            self.runtime.handle().clone(),
        ))
    }
}

impl fmt::Debug for ExecResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult")
            .field("id", &self.inner.id)
            .finish()
    }
}
