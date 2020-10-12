use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use shiplift::tty::TtyChunk;

// may not be needed
/// Defines error cases when waiting for a message in a stream.
#[derive(Debug)]
pub enum WaitErrorAsync {
    EndOfStream(),
    Docker(shiplift::Error),
}

/// Extension trait for LogStream to wait for a message to appear in the given stream.
#[async_trait]
pub trait WaitForMessageAsync {
    async fn wait_for_message_async(&mut self, message: &str) -> Result<(), WaitErrorAsync>;
}

#[async_trait]
impl<T> WaitForMessageAsync for T
where
    T: Stream<Item = Result<TtyChunk, shiplift::Error>> + Unpin + Send,
{
    async fn wait_for_message_async(&mut self, message: &str) -> Result<(), WaitErrorAsync> {
        while let Some(log_result) = self.next().await {
            match log_result {
                Ok(chunk) => match chunk {
                    TtyChunk::StdOut(bytes) => {
                        let line = std::str::from_utf8(&bytes).unwrap();
                        if line.contains(message) {
                            log::info!("Found message");
                            return Ok(());
                        }
                    }
                    TtyChunk::StdErr(bytes) => {
                        let line = std::str::from_utf8(&bytes).unwrap();
                        if line.contains(message) {
                            log::info!("Found message");
                            return Ok(());
                        }
                    }
                    TtyChunk::StdIn(_) => unreachable!(),
                },
                Err(e) => {
                    return Err(WaitErrorAsync::Docker(e));
                }
            }
        }

        log::error!("Failed to find message in stream",);
        Err(WaitErrorAsync::EndOfStream())
    }
}
