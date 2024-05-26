use std::{
    io::{BufRead, Read},
    sync::Arc,
};

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt};

/// Allows to use [`tokio::io::AsyncRead`] synchronously as [`std::io::Read`].
/// In fact, it's almost the same as [`tokio_util::io::SyncIoBridge`], but utilizes [`tokio::runtime::Runtime`] instead of [`tokio::runtime::Handle`].
/// This is needed because [`tokio::runtime::Handle::block_on`] can't drive the IO on `current_thread` runtime.
pub(super) struct SyncReadBridge<T> {
    inner: T,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl<T: Unpin> SyncReadBridge<T> {
    pub fn new(inner: T, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self { inner, runtime }
    }
}

impl<T: AsyncBufRead + Unpin> BufRead for SyncReadBridge<T> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let inner = &mut self.inner;
        self.runtime.block_on(AsyncBufReadExt::fill_buf(inner))
    }

    fn consume(&mut self, amt: usize) {
        let inner = &mut self.inner;
        AsyncBufReadExt::consume(inner, amt)
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let inner = &mut self.inner;
        self.runtime
            .block_on(AsyncBufReadExt::read_until(inner, byte, buf))
    }
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        let inner = &mut self.inner;
        self.runtime
            .block_on(AsyncBufReadExt::read_line(inner, buf))
    }
}

impl<T: AsyncRead + Unpin> Read for SyncReadBridge<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let inner = &mut self.inner;
        self.runtime.block_on(AsyncReadExt::read(inner, buf))
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let inner = &mut self.inner;
        self.runtime.block_on(inner.read_to_end(buf))
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        let inner = &mut self.inner;
        self.runtime.block_on(inner.read_to_string(buf))
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let inner = &mut self.inner;
        // The AsyncRead trait returns the count, synchronous doesn't.
        let _n = self.runtime.block_on(inner.read_exact(buf))?;
        Ok(())
    }
}
