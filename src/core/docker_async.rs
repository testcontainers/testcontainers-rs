use crate::core::{Logs, Ports, RunArgs};
use crate::Image;
use async_trait::async_trait;

#[async_trait]
pub trait DockerAsync
where
    Self: Sized,
{
    async fn run<I: Image + Send>(&self, image: I) -> ContainerAsync<'_, Self, I>;
    async fn run_with_args<I: Image + Send>(
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

/// Represents a running docker container using async trait.
///
/// Containers have a [`custom destructor`][drop_impl] that removes them as soon as they go out of scope:
///
/// ```rust
/// use testcontainers::*;
/// #[test]
/// fn a_test() {
///     let docker = clients::Shiplift::default();
///
///     {
///         let container = docker.run(MyImage::default());
///
///         // Docker container is stopped/removed at the end of this scope.
///     }
/// }
///
/// ```
///
/// [drop_impl]: struct.Container.html#impl-Drop
#[derive(Debug)]
pub struct ContainerAsync<'d, D, I>
where
    D: DockerAsync,
    I: Image,
{
    id: String,
    docker_client: &'d D,
    image: I,
}

impl<'d, D, I> ContainerAsync<'d, D, I>
where
    D: DockerAsync,
    I: Image,
{
    /// Constructs a new container given an id, a docker client and the image.
    pub fn new(id: String, docker_client: &'d D, image: I) -> Self {
        let container = ContainerAsync {
            id,
            docker_client,
            image,
        };

        // container.block_until_ready();

        container
    }

    async fn block_until_ready(&self) {
        log::debug!("Waiting for container {} to be ready", self.id);

        // self.image.wait_until_ready_async(self).await;

        log::debug!("Container {} is now ready!", self.id);
    }
}
