use crate::{core::copy::CopyToContainerCollection, Image};

/// Trait for images that can be built from within your tests or testcontainer libraries.
///   
/// Unlike the [`Image`] trait which represents existing Docker images, `BuildableImage`   
/// represents images that need to be constructed from a, possibly even dynamic, `Dockerfile``
/// and the needed Docker build context.
///
/// If you want to dynamically create Dockerfiles look at Dockerfile generator crates like:
/// <https://crates.io/crates/dockerfile_builder>
///   
/// The build process, executed by [`crate::runners::SyncBuilder`] / [`crate::runners::AsyncBuilder`], follows these steps:  
/// 1. Collect build context via `build_context()` which will be tarred and sent to buildkit.
/// 2. Generate image descriptor via `descriptor()` which will be passed to the container
/// 3. Build the Docker image using the Docker API
/// 4. Convert to runnable [`Image`] via `into_image()` which consumes the `BuildableImage`
///    into an `Image`
///   
/// # Example  
///   
/// ```rust  
/// use testcontainers::{GenericBuildableImage, runners::AsyncBuilder};  
///
/// #[tokio::test]
/// async fn test_example() -> anyhow::Result<()> {
///     let image = GenericBuildableImage::new("example-tc", "1.0")
///         .with_dockerfile_string("FROM alpine:latest\nRUN echo 'hello'")
///         .build_image().await?;
///     // start container
///     // use it
/// }
/// ```  
///   
pub trait BuildableImage {
    /// The type of [`Image`] that this buildable image produces after building.
    type Built: Image;

    /// Returns the build context containing all files and data needed to build the image.
    ///
    /// The build context consist of at least the `Dockerfile` and needs all the resources
    /// referred to by the Dockerfile.
    /// This is more or less equivalent to the directory you would pass to `docker build`.
    ///
    /// <https://docs.docker.com/build/concepts/context/>
    ///
    /// For creating build contexts, use the [`crate::core::BuildContextBuilder`] API when not using
    /// [`crate::GenericBuildableImage`], which wraps `BuildContextBuilder` builder functions.
    ///   
    /// # Returns  
    ///   
    /// A [`CopyToContainerCollection`] containing the build context in a form we
    /// can send it to buildkit.  
    fn build_context(&self) -> CopyToContainerCollection;

    /// Returns the image descriptor (name:tag) that will be assigned to the built image and be
    /// passed down to the container for running.
    ///
    /// # Returns
    ///
    /// A string in the format "name:tag" that uniquely identifies the built image.
    fn descriptor(&self) -> String;

    /// Consumes this buildable image and converts it into a runnable [`Image`].
    ///
    /// This method is called after the Docker image has been successfully built.
    /// It transforms the build specification into a standard [`Image`] that can be
    /// started as a container.
    ///
    /// # Returns
    ///
    /// An [`Image`] instance configured to run the built Docker image.
    fn into_image(self) -> Self::Built;
}
