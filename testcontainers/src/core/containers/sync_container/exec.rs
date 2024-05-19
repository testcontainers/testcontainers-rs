use std::{fmt, io, io::BufRead};

use crate::core::sync_container::sync_reader;

/// Represents the result of an executed command in a container.
pub struct SyncExecResult<'a> {
    pub(super) inner: crate::core::async_container::exec::ExecResult<'a>,
    pub(super) runtime: &'a tokio::runtime::Runtime,
}

impl<'a> SyncExecResult<'a> {
    /// Returns the exit code of the executed command.
    /// If the command has not yet exited, this will return `None`.
    pub fn exit_code(&self) -> Result<Option<i64>, bollard::errors::Error> {
        self.runtime.block_on(self.inner.exit_code())
    }

    /// Returns an asynchronous reader for stdout.
    pub fn stdout<'b>(&'b mut self) -> Box<dyn BufRead + 'b> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.inner.stdout(),
            self.runtime,
        ))
    }

    /// Returns an asynchronous reader for stderr.
    pub fn stderr<'b>(&'b mut self) -> Box<dyn BufRead + 'b> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.inner.stderr(),
            self.runtime,
        ))
    }

    /// Returns stdout as a vector of bytes.
    pub fn stdout_to_vec(&mut self) -> Result<Vec<u8>, io::Error> {
        self.runtime.block_on(self.inner.stdout_to_vec())
    }

    /// Returns stderr as a vector of bytes.
    pub fn stderr_to_vec(&mut self) -> Result<Vec<u8>, io::Error> {
        self.runtime.block_on(self.inner.stderr_to_vec())
    }
}

impl fmt::Debug for SyncExecResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult")
            .field("id", &self.inner.id)
            .finish()
    }
}
