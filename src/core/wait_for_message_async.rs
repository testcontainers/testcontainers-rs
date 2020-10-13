use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;

// may not be needed
/// Defines error cases when waiting for a message in a stream.
#[derive(Debug)]
pub enum WaitErrorAsync {
    EndOfStream(),
    IO(std::io::Error),
}

/// Extension trait for LogStream to wait for a message to appear in the given stream.
#[async_trait]
pub trait WaitForMessageAsync {
    async fn wait_for_message_async(&mut self, message: &str) -> Result<(), WaitErrorAsync>;
}

#[async_trait]
impl<T> WaitForMessageAsync for T
where
    T: Stream<Item = Vec<u8>> + Unpin + Send,
{
    async fn wait_for_message_async(&mut self, message: &str) -> Result<(), WaitErrorAsync> {
        while let Some(bytes) = self.next().await {
            let line = std::str::from_utf8(&bytes).unwrap();
            if line.contains(message) {
                log::info!("Found message");
                return Ok(());
            }
        }

        log::error!("Failed to find message in stream",);
        Err(WaitErrorAsync::EndOfStream())
    }
}
