use std::{borrow::Cow, fmt::Debug};

pub use exec::ExecCommand;
pub use image_ext::ImageExt;
#[cfg(feature = "reusable-containers")]
pub use image_ext::ReuseDirective;
use url::Host;

use crate::{
    core::{
        copy::CopyToContainer,
        error::Result,
        mounts::Mount,
        ports::{ContainerPort, Ports},
        WaitFor,
    },
    ContainerAsync, TestcontainersError,
};

mod exec;
mod image_ext;

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
{
    /// The name of the docker image to pull from the Docker Hub registry.
    fn name(&self) -> &str;

    /// Implementations are encouraged to include a tag that will not change (i.e. NOT latest)
    /// in order to prevent test code from randomly breaking because the underlying docker
    /// suddenly changed.
    fn tag(&self) -> &str;

    /// Returns a list of conditions that need to be met before a started container is considered ready.
    ///
    /// This method is the **🍞 and butter** of the whole testcontainers library. Containers are
    /// rarely instantly available as soon as they are started. Most of them take some time to boot
    /// up.
    ///
    /// The conditions returned from this method are evaluated **in the order** they are returned. Therefore
    /// you most likely want to start with a [`WaitFor::Log`] or [`WaitFor::Http`].
    fn ready_conditions(&self) -> Vec<WaitFor>;

    /// Returns the environment variables that needs to be set when a container is created.
    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        std::iter::empty::<(String, String)>()
    }

    /// Returns the mounts that needs to be created when a container is created.
    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        std::iter::empty()
    }

    /// Returns the files to be copied into the container at startup.
    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        std::iter::empty()
    }

    /// Returns the [entrypoint](`https://docs.docker.com/reference/dockerfile/#entrypoint`) this image needs to be created with.
    fn entrypoint(&self) -> Option<&str> {
        None
    }

    /// Returns the [`CMD`](https://docs.docker.com/reference/dockerfile/#cmd) this image needs to be created with.
    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        std::iter::empty::<String>()
    }

    /// Returns the ports that needs to be exposed when a container is created.
    ///
    /// This method is useful when there is a need to expose some ports, but there is
    /// no `EXPOSE` instruction in the Dockerfile of an image.
    fn expose_ports(&self) -> &[ContainerPort] {
        &[]
    }

    /// Returns the commands that needs to be executed after a container is started i.e. commands
    /// to be run in a running container.
    ///
    /// Notice, that you can return an error from this method, for example if container's state is unexpected.
    /// In this case, you can use `TestcontainersError::other` to wrap an arbitrary error.
    ///
    /// This method is useful when certain re-configuration is required after the start
    /// of container for the container to be considered ready for use in tests.
    #[allow(unused_variables)]
    fn exec_after_start(&self, cs: ContainerState) -> Result<Vec<ExecCommand>> {
        Ok(Default::default())
    }

    /// Returns commands that will be executed after the container has started, but before the
    /// [Image::ready_conditions] are awaited for.
    ///
    /// Use this when you, e.g., need to configure something based on the container's ports and host
    /// (for example an application that needs to know its own address).
    #[allow(unused_variables)]
    fn exec_before_ready(&self, cs: ContainerState) -> Result<Vec<ExecCommand>> {
        Ok(Default::default())
    }
}

#[derive(Debug)]
pub struct ContainerState {
    id: String,
    host: Host,
    ports: Ports,
}

impl ContainerState {
    pub async fn from_container<I>(container: &ContainerAsync<I>) -> Result<Self>
    where
        I: Image,
    {
        Ok(Self {
            id: container.id().into(),
            host: container.get_host().await?,
            ports: container.ports().await?,
        })
    }

    pub fn host(&self) -> &Host {
        &self.host
    }

    /// Returns the host port for the given internal container's port (`IPv4`).
    ///
    /// Results in an error ([`TestcontainersError::PortNotExposed`]) if the port is not exposed.
    pub fn host_port_ipv4(&self, internal_port: ContainerPort) -> Result<u16> {
        self.ports
            .map_to_host_port_ipv4(internal_port)
            .ok_or_else(|| TestcontainersError::PortNotExposed {
                id: self.id.clone(),
                port: internal_port,
            })
    }

    /// Returns the host port for the given internal container's port (`IPv6`).
    ///
    /// Results in an error ([`TestcontainersError::PortNotExposed`]) if the port is not exposed.
    pub fn host_port_ipv6(&self, internal_port: ContainerPort) -> Result<u16> {
        self.ports
            .map_to_host_port_ipv6(internal_port)
            .ok_or_else(|| TestcontainersError::PortNotExposed {
                id: self.id.clone(),
                port: internal_port,
            })
    }
}
