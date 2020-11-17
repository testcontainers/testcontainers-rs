use crate::core::{Container, Docker};

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
    Self: Sized + Default,
    Self::Args: Default + IntoIterator<Item = String>,
    Self::EnvVars: Default + IntoIterator<Item = (String, String)>,
    Self::Volumes: Default + IntoIterator<Item = (String, String)>,
    Self::EntryPoint: ToString,
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

    /// A type representing the environment variables for an Image.
    ///
    /// There are a couple of things regarding the arguments of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its environment variables should be meaningful!
    /// 2. Implementations should be conservative about which environment variables they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the environment variables of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    type EnvVars;

    /// A type representing the volumes for an Image.
    ///
    /// There are a couple of things regarding the arguments of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its volumes should be meaningful!
    /// 2. Implementations should be conservative about which volumes they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the volumes of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    type Volumes;

    /// A type representing the entrypoint for an Image.
    type EntryPoint: ?Sized;

    /// The descriptor of the docker image.
    ///
    /// This should return a full-qualified descriptor.
    /// Implementations are encouraged to include a tag that will not change (i.e. NOT latest)
    /// in order to prevent test code from randomly breaking because the underlying docker
    /// suddenly changed.
    fn descriptor(&self) -> String;

    /// Blocks the current thread until the started container is ready.
    ///
    /// This method is the **🍞 and butter** of the whole testcontainers library. Containers are
    /// rarely instantly available as soon as they are started. Most of them take some time to boot
    /// up.
    ///
    /// Implementations MUST block the current thread until the passed-in container is ready to be
    /// interacted with. The container instance provides access to logs of the container.
    ///
    /// Most implementations will very likely want to make use of this to wait for a particular
    /// message to be emitted.
    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>);

    /// Returns the arguments this instance was created with.
    fn args(&self) -> Self::Args;

    /// Returns the environment variables this instance was created with.
    fn env_vars(&self) -> Self::EnvVars;

    /// Returns the volumes this instance was created with.
    fn volumes(&self) -> Self::Volumes;

    /// Re-configures the current instance of this image with the given arguments.
    fn with_args(self, arguments: Self::Args) -> Self;

    /// Re-configures the current instance of this image with the given entrypoint.
    fn with_entrypoint(self, _entryppoint: &Self::EntryPoint) -> Self {
        self
    }

    /// Returns the entrypoint this instance was created with.
    fn entrypoint(&self) -> Option<String> {
        None
    }
}

/// Represents a port mapping between a local port and the internal port of a container.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub local: u16,
    pub internal: u16,
}

impl Into<Port> for (u16, u16) {
    fn into(self) -> Port {
        Port {
            local: self.0,
            internal: self.1,
        }
    }
}
