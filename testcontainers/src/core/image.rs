use std::fmt::Debug;

pub use exec_command::ExecCommand;
pub use runnable_image::{CgroupnsMode, Host, PortMapping, RunnableImage};
pub use wait_for::WaitFor;

use super::ports::Ports;
use crate::core::mounts::Mount;

mod exec_command;
mod runnable_image;
mod wait_for;

/// Represents a docker image.
///
/// Implementations are required to implement Default. The default instance of an [`Image`]
/// should have a meaningful configuration! It should be possible to [`run`][docker_run] the default
/// instance of an Image and get back a working container!
///
/// [`Image`]: trait.Image.html
/// [docker_run]: trait.Docker.html#tymethod.run
pub trait Image
where
    Self: Sized + Sync + Send,
    Self::Args: ImageArgs + Clone + Debug + Sync + Send,
{
    /// A type representing the arguments for an Image.
    ///
    /// There are a couple of things regarding the arguments of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its arguments should be meaningful!
    /// 2. Implementations should be conservative about which arguments they expose. Many times,
    /// users will either go with the default arguments or just override one or two. When defining
    /// the arguments of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    type Args;

    /// The name of the docker image to pull from the Docker Hub registry.
    fn name(&self) -> String;

    /// Implementations are encouraged to include a tag that will not change (i.e. NOT latest)
    /// in order to prevent test code from randomly breaking because the underlying docker
    /// suddenly changed.
    fn tag(&self) -> String;

    /// Returns a list of conditions that need to be met before a started container is considered ready.
    ///
    /// This method is the **🍞 and butter** of the whole testcontainers library. Containers are
    /// rarely instantly available as soon as they are started. Most of them take some time to boot
    /// up.
    ///
    /// The conditions returned from this method are evaluated **in the order** they are returned. Therefore
    /// you most likely want to start with a [`WaitFor::StdOutMessage`] or [`WaitFor::StdErrMessage`] and
    /// potentially follow up with a [`WaitFor::Duration`] in case the container usually needs a little
    /// more time before it is ready.
    fn ready_conditions(&self) -> Vec<WaitFor>;

    /// There are a couple of things regarding the environment variables of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its environment variables should be meaningful!
    /// 2. Implementations should be conservative about which environment variables they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the environment variables of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(std::iter::empty())
    }

    /// There are a couple of things regarding the mounts of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its mounts should be meaningful!
    /// 2. Implementations should be conservative about which mounts they expose or require. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the volumes of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    fn mounts(&self) -> Box<dyn Iterator<Item = &Mount> + '_> {
        Box::new(std::iter::empty())
    }

    /// Returns the entrypoint this instance was created with.
    fn entrypoint(&self) -> Option<String> {
        None
    }

    /// Returns the ports that needs to be exposed when a container is created.
    ///
    /// This method is useful when there is a need to expose some ports, but there is
    /// no EXPOSE instruction in the Dockerfile of an image.
    fn expose_ports(&self) -> Vec<u16> {
        Default::default()
    }

    /// Returns the commands that needs to be executed after a container is started i.e. commands
    /// to be run in a running container.
    ///
    /// This method is useful when certain re-configuration is required after the start
    /// of container for the container to be considered ready for use in tests.
    #[allow(unused_variables)]
    fn exec_after_start(&self, cs: ContainerState) -> Vec<ExecCommand> {
        Default::default()
    }
}

#[derive(Debug)]
pub struct ContainerState {
    ports: Ports,
}

impl ContainerState {
    pub fn new(ports: Ports) -> Self {
        Self { ports }
    }

    pub fn host_port_ipv4(&self, internal_port: u16) -> u16 {
        self.ports
            .map_to_host_port_ipv4(internal_port)
            .unwrap_or_else(|| panic!("Container does not have a mapped port for {internal_port}",))
    }

    pub fn host_port_ipv6(&self, internal_port: u16) -> u16 {
        self.ports
            .map_to_host_port_ipv6(internal_port)
            .unwrap_or_else(|| panic!("Container does not have a mapped port for {internal_port}",))
    }
}

pub trait ImageArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>>;
}

impl ImageArgs for () {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(vec![].into_iter())
    }
}
