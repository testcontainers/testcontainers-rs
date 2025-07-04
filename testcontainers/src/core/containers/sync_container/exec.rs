use std::{fmt, io::BufRead, sync::Arc};

use crate::{
    core::{async_container, sync_container::sync_reader},
    TestcontainersError,
};

/// Represents the result of an executed command in a container.
pub struct SyncExecResult {
    pub(super) inner: async_container::exec::ExecResult,
    pub(super) runtime: Arc<tokio::runtime::Runtime>,
}

impl SyncExecResult {
    /// Returns the exit code of the executed command.
    /// If the command has not yet exited, this will return `None`.
    pub fn exit_code(&self) -> Result<Option<i64>, TestcontainersError> {
        self.runtime.block_on(self.inner.exit_code())
    }

    /// Returns an asynchronous reader for stdout.
    pub fn stdout<'b>(&'b mut self) -> Box<dyn BufRead + Send + 'b> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.inner.stdout(),
            self.runtime.clone(),
        ))
    }

    /// Returns an asynchronous reader for stderr.
    pub fn stderr<'b>(&'b mut self) -> Box<dyn BufRead + Send + 'b> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.inner.stderr(),
            self.runtime.clone(),
        ))
    }

    /// Returns stdout as a vector of bytes.
    /// Keep in mind that this will block until the command exits.
    ///
    /// If you want to read stderr in chunks, use [`SyncExecResult::stdout`] instead.
    pub fn stdout_to_vec(&mut self) -> Result<Vec<u8>, TestcontainersError> {
        self.runtime.block_on(self.inner.stdout_to_vec())
    }

    /// Returns stderr as a vector of bytes.
    /// Keep in mind that this will block until the command exits.
    ///
    /// If you want to read stderr in chunks, use [`SyncExecResult::stderr`] instead.
    pub fn stderr_to_vec(&mut self) -> Result<Vec<u8>, TestcontainersError> {
        self.runtime.block_on(self.inner.stderr_to_vec())
    }
}

impl fmt::Debug for SyncExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecResult")
            .field("id", &self.inner.id)
            .finish()
    }
}
