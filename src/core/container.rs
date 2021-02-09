use crate::{
    core::{docker::DockerOps, env::Command, image::WaitFor, Logs},
    Image, WaitForMessage,
};
use std::{fmt, marker::PhantomData, sync::Arc};

/// Represents a running docker container.
///
/// Containers have a [`custom destructor`][drop_impl] that removes them as soon as they go out of scope:
///
/// ```rust
/// use testcontainers::*;
/// #[test]
/// fn a_test() {
///     let docker = clients::Cli::default();
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
pub struct Container<'d, I> {
    id: String,
    docker: Arc<dyn DockerOps>,
    image: I,
    command: Command,
    /// Keeps track of the clients lifetime.
    ///
    /// Internally we use an Arc to access the client but we still want to make sure that the client outlives the docker container.
    _phantom: PhantomData<&'d ()>,
}

impl<'d, I> fmt::Debug for Container<'d, I>
where
    I: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Container")
            .field("id", &self.id)
            .field("image", &self.image)
            .field("command", &self.command)
            .finish()
    }
}

impl<'d, I> Container<'d, I>
where
    I: Image,
{
    /// Constructs a new container given an id, a docker client and the image.
    ///
    /// This function will block the current thread (if [`wait_until_ready`] is implemented correctly) until the container is actually ready to be used.
    ///
    /// [`wait_until_ready`]: trait.Image.html#tymethod.wait_until_ready
    pub(crate) fn new<O>(id: String, docker_client: &'d Arc<O>, image: I, command: Command) -> Self
    where
        O: DockerOps + 'static,
    {
        let container = Container {
            id,
            docker: docker_client.clone(),
            image,
            command,
            _phantom: PhantomData,
        };

        container.block_until_ready();

        container
    }

    /// Returns a reference to the [`Image`] of this container.
    ///
    /// Access to this is useful if the [`arguments`] of the [`Image`] change how to connect to the
    /// Access to this is useful to retrieve [`Image`] specific information such as authentication details or other relevant information which have been passed as [`arguments`]
    ///
    /// [`Image`]: trait.Image.html
    /// [`arguments`]: trait.Image.html#associatedtype.Args
    pub fn image(&self) -> &I {
        &self.image
    }

    fn block_until_ready(&self) {
        log::debug!("Waiting for container {} to be ready", self.id);

        for condition in self.image.ready_conditions() {
            match condition {
                WaitFor::StdOutMessage { message } => {
                    self.logs().stdout.wait_for_message(&message).unwrap()
                }
                WaitFor::StdErrMessage { message } => {
                    self.logs().stderr.wait_for_message(&message).unwrap()
                }
                WaitFor::Duration { length } => {
                    std::thread::sleep(length);
                }
                WaitFor::Nothing => {}
            }
        }

        log::debug!("Container {} is now ready!", self.id);
    }
}

impl<'d, I> Container<'d, I> {
    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Gives access to the log streams of this container.
    pub fn logs(&self) -> Logs {
        self.docker.logs(&self.id)
    }

    /// Returns the mapped host port for an internal port of this docker container.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will panic.
    ///
    /// # Panics
    ///
    /// This method panics if the given port is not mapped.
    /// Testcontainers is designed to be used in tests only. If a certain port is not mapped, the container
    /// is unlikely to be useful.
    pub fn get_host_port(&self, internal_port: u16) -> u16 {
        self.docker
            .ports(&self.id)
            .map_to_host_port(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "container {} does not expose port {}",
                    self.id, internal_port
                )
            })
    }

    pub fn stop(&self) {
        log::debug!("Stopping docker container {}", self.id);

        self.docker.stop(&self.id)
    }

    pub fn start(&self) {
        self.docker.start(&self.id);
    }

    pub fn rm(&self) {
        log::debug!("Deleting docker container {}", self.id);

        self.docker.rm(&self.id)
    }
}

/// The destructor implementation for a Container.
///
/// As soon as the container goes out of scope, the destructor will either only stop or delete the docker container, depending on the [`Command`] value.
///
/// Setting it to `keep` will stop container.
/// Setting it to `remove` will remove it.
impl<'d, I> Drop for Container<'d, I> {
    fn drop(&mut self) {
        match self.command {
            Command::Keep => {}
            Command::Remove => self.rm(),
        }
    }
}
