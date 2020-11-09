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

/// Log streams of running container (Stream<Item = Vec<u8>>).
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct LogsAsync<'a> {
    #[derivative(Debug = "ignore")]
    pub stdout: Box<dyn Stream<Item = Vec<u8>> + Unpin + Send + 'a>,
    #[derivative(Debug = "ignore")]
    pub stderr: Box<dyn Stream<Item = Vec<u8>> + Unpin + Send + 'a>,
}
