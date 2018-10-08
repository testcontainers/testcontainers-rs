use Container;
use Docker;

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

    /// The descriptor of the docker image.
    ///
    /// This should return a full-qualified descriptor.
    /// Implementations are encouraged to include a tag that will not change (i.e. NOT latest)
    /// in order to prevent test code from randomly breaking because the underlying docker
    /// suddenly changed.
    fn descriptor(&self) -> String;

    /// Blocks the current thread until the started container is ready.
    ///
    /// This method is the **bread and butter** of the whole testcontainers library. Containers are
    /// rarely instantly available as soon as they are started. Most of them take some time to boot
    /// up.
    ///
    /// Implementations MUST block the current thread until the passed-in container is ready to be
    /// interacted with. The container instance provides access to logs of the container.
    ///
    /// Most implementations will very likely want to make use of this to wait for a particular
    /// message to be emitted.
    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>);

    /// Returns the arguments this instance was created with.
    fn args(&self) -> Self::Args;

    /// Re-configures the current instance of this image with the given arguments.
    fn with_args(self, arguments: Self::Args) -> Self;
}
