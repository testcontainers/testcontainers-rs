use crate::core::{ContainerAsync, ImageAsync, Ports, RunArgs};
use async_trait::async_trait;
use futures::Stream;

#[async_trait]
pub trait DockerAsync
where
    Self: Sized,
    Self: Sync,
{
    async fn run<I: ImageAsync + Sync>(&self, image: I) -> ContainerAsync<'_, Self, I>;
    async fn run_with_args<I: ImageAsync + Send + Sync>(
        &self,
        image: I,
        run_args: RunArgs,
    ) -> ContainerAsync<'_, Self, I>;

    async fn logs<'a>(&'a self, id: &'a str) -> LogsAsync<'a>;
    async fn ports(&self, id: &str) -> Ports;
    async fn rm(&self, id: &str);
    async fn stop(&self, id: &str);
    async fn start(&self, id: &str);
}

/// Log streams of running container (Stream<Item=TtyChunk>).
/// Instead of wrapping around the interface, just expose the underlying
/// shiplift API because it's usually used by wait_for_message
/// providing reference implementation in addition to that should be
/// enough for most use cases
/// AsyncRead is not as established as std::io::read and due to the
/// special TtyChunk handling in shiplift, it takes some decoding too
// type LogStream = Box<dyn Stream<Item = Result<TtyChunk, shiplift::Error>> + Unpin>;
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct LogsAsync<'a> {
    #[derivative(Debug = "ignore")]
    pub stdout: Box<dyn Stream<Item = Vec<u8>> + Unpin + Send + 'a>,
    #[derivative(Debug = "ignore")]
    pub stderr: Box<dyn Stream<Item = Vec<u8>> + Unpin + Send + 'a>,
}
