use crate::core::{DockerAsync, ImageAsync};

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
    pub async fn logs(&'static self) -> <D as DockerAsync>::LogStream {
        self.docker_client.logs(&self.id).await
    }

    async fn block_until_ready(&self) {
        log::debug!("Waiting for container {} to be ready", self.id);

        self.image.wait_until_ready(self).await;

        log::debug!("Container {} is now ready!", self.id);
    }
}
