use crate::core::{ContainerAsync, ImageAsync, Logs, Ports, RunArgs};
use async_trait::async_trait;

#[async_trait]
pub trait DockerAsync
where
    Self: Sized,
{
    async fn run<I: ImageAsync + Sync>(&self, image: I) -> ContainerAsync<'_, Self, I>;
    async fn run_with_args<I: ImageAsync + Send + Sync>(
        &self,
        image: I,
        run_args: RunArgs,
    ) -> ContainerAsync<'_, Self, I>;
    async fn logs(&self, id: &str) -> Logs;
    async fn ports(&self, id: &str) -> Ports;
    async fn rm(&self, id: &str);
    async fn stop(&self, id: &str);
    async fn start(&self, id: &str);
}
