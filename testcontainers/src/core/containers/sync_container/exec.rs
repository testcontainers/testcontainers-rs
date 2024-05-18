use std::{fmt, io};

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

    /// Returns stdout as a vector of bytes.
    pub fn stdout(&mut self) -> Result<Vec<u8>, io::Error> {
        self.runtime.block_on(self.inner.stdout())
    }

    /// Returns stderr as a vector of bytes.
    pub fn stderr(&mut self) -> Result<Vec<u8>, io::Error> {
        self.runtime.block_on(self.inner.stderr())
    }
}

impl fmt::Debug for SyncExecResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult")
            .field("id", &self.inner.id)
            .finish()
    }
}
