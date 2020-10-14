use crate::core::{DockerAsync, ImageAsync, LogsAsync};
use std::env::var;

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
    D: DockerAsync + Sync,
    I: ImageAsync + Send,
{
    id: String,
    docker_client: &'d D,
    image: I,
}

impl<'d, D, I> ContainerAsync<'d, D, I>
where
    D: DockerAsync + Sync,
    I: ImageAsync + Send,
{
    /// Constructs a new container given an id, a docker client and the image.
    /// ContainerAsync::new().await
    /// XXX It's a bit weird to have the new method not immediately return
    pub async fn new(id: String, docker_client: &'d D, image: I) -> ContainerAsync<'d, D, I> {
        let container = ContainerAsync {
            id,
            docker_client,
            image,
        };

        container.block_until_ready().await;

        container
    }

    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Gives access to the log streams of this container.
    pub async fn logs<'a>(&'a self) -> LogsAsync<'a> {
        let id_clone: String = String::from(&self.id);
        self.docker_client
            .logs(Box::leak(id_clone.into_boxed_str()))
            .await
    }

    async fn block_until_ready(&self) {
        log::debug!("Waiting for container {} to be ready", self.id);

        self.image.wait_until_ready(self).await;

        log::debug!("Container {} is now ready!", self.id);
    }

    pub async fn stop(&self) {
        log::debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id).await
    }

    pub async fn start(&self) {
        self.docker_client.start(&self.id).await
    }

    pub async fn rm(&self) {
        log::debug!("Deleting docker container {}", self.id);

        self.docker_client.rm(&self.id).await
    }

    async fn drop_async(&self) {
        let keep_container = var("KEEP_CONTAINERS")
            .ok()
            .and_then(|var| var.parse().ok())
            .unwrap_or(false);

        if keep_container {
            self.stop().await
        } else {
            self.rm().await
        }
    }
}

use futures::executor::block_on;

/// The destructor implementation for a Container.
///
/// As soon as the container goes out of scope, the destructor will either only stop or delete the docker container.
/// This behaviour can be controlled through the `KEEP_CONTAINERS` environment variable. Setting it to `true` will only stop containers instead of removing them. Any other or no value will remove the container.
impl<'d, D, I> Drop for ContainerAsync<'d, D, I>
where
    D: DockerAsync,
    I: ImageAsync,
{
    fn drop(&mut self) {
        block_on(self.drop_async());
    }
}
