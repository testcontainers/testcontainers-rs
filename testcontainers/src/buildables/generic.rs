use std::path::PathBuf;

use crate::{
    core::{copy::CopyToContainerCollection, BuildContextBuilder},
    BuildableImage, GenericImage,
};

/// A generic implementation of [`BuildableImage`] for building custom Docker images.
///
/// `GenericBuildableImage` provides a fluent interface for constructing Docker images from
/// Dockerfiles and build contexts. It supports adding files and directories from the filesystem,
/// embedding data directly, and customizing the build process.
///
/// # Build Context Management
///   
/// The build context is managed through a [`BuildContextBuilder`] that collects all files
/// and data needed for the Docker build. Files are automatically packaged into a TAR archive
/// that gets sent to the Docker daemon.
///   
/// # Example: Basic Image Build  
///   
/// ```rust,no_run
/// use testcontainers::{GenericBuildableImage, runners::AsyncBuilder};
///
/// #[tokio::test]
/// async fn test_hello() -> anyhow::Result<()> {
///     let image = GenericBuildableImage::new("hello-world", "latest")
///         .with_dockerfile_string(
///             r#"FROM alpine:latest
///             COPY hello.sh /usr/local/bin/
///             RUN chmod +x /usr/local/bin/hello.sh
///             CMD ["/usr/local/bin/hello.sh"]"#
///         )
///         .with_data(
///             "#!/bin/sh\necho 'Hello from custom image!'",
///             "./hello.sh"
///         )
///         .build_image().await?;
///     // start container
///     // use it
/// }
/// ```
///   
/// # Example: Multi-File Build Context  
///   
/// ```rust,no_run
/// use testcontainers::{GenericBuildableImage, runners::AsyncBuilder};
///
/// #[tokio::test]
/// async fn test_webapp() -> anyhow::Result<()>  {
///     let image = GenericBuildableImage::new("web-app", "1.0")
///         .with_dockerfile("./Dockerfile")
///         .with_file("./package.json", "./package.json")
///         .with_file("./src", "./src")
///         .with_data(vec![0x00, 0x01, 0x02], "./data.dat")
///         .build_image().await?;
///     // start container
///     // use it
/// }
/// ```  
#[derive(Debug)]
pub struct GenericBuildableImage {
    /// The name of the Docker image to be built
    name: String,
    /// The tag assigned to the built image and passed down to the [`Image`]
    tag: String,
    /// Wrapped builder for managing the build context
    build_context_builder: BuildContextBuilder,
}

impl GenericBuildableImage {
    /// Creates a new buildable image with the specified name and tag.
    ///
    /// # Arguments
    ///
    /// * `name` - The name for the Docker image (e.g., "my-app", "registry.com/service")
    /// * `tag` - The tag for the image (e.g., "latest", "1.0", "dev")
    pub fn new(name: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tag: tag.into(),
            build_context_builder: BuildContextBuilder::default(),
        }
    }

    /// Adds a Dockerfile from the filesystem to the build context.
    ///
    /// # Arguments
    ///
    /// * `source` - Path to the Dockerfile on the local filesystem
    pub fn with_dockerfile(mut self, source: impl Into<PathBuf>) -> Self {
        self.build_context_builder = self.build_context_builder.with_dockerfile(source);
        self
    }

    /// Adds a Dockerfile from a string to the build context.
    ///
    /// This is useful for generating Dockerfiles programmatically or embedding
    /// simple Dockerfiles directly in test code.
    ///
    /// # Arguments
    ///
    /// * `content` - The complete Dockerfile content as a string
    pub fn with_dockerfile_string(mut self, content: impl Into<String>) -> Self {
        self.build_context_builder = self.build_context_builder.with_dockerfile_string(content);
        self
    }

    /// Adds a file or directory from the filesystem to the build context.
    ///
    /// Be aware, that if you don't add the Dockerfile with the specific `with_dockerfile()`
    /// or `with_dockerfile_string()` functions it has to be named `Dockerfile`in the build
    /// context. Containerfile won't be recognized!
    ///
    /// # Arguments
    ///
    /// * `source` - Path to the file or directory on the local filesystem
    /// * `target` - Path where the file should be placed in the build context
    pub fn with_file(mut self, source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        self.build_context_builder = self.build_context_builder.with_file(source, target);
        self
    }

    /// Adds data directly to the build context as a file.
    ///
    /// This method allows you to embed file content directly without requiring
    /// files to exist on the filesystem. Useful for generated content, templates,
    /// or small configuration files.
    ///
    /// # Arguments
    ///
    /// * `data` - The file content as bytes
    /// * `target` - Path where the file should be placed in the build context
    pub fn with_data(mut self, data: impl Into<Vec<u8>>, target: impl Into<String>) -> Self {
        self.build_context_builder = self.build_context_builder.with_data(data, target);
        self
    }
}

impl BuildableImage for GenericBuildableImage {
    type Built = GenericImage;

    fn build_context(&self) -> CopyToContainerCollection {
        self.build_context_builder.as_copy_to_container_collection()
    }

    fn descriptor(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }

    fn into_image(self) -> Self::Built {
        GenericImage::new(&self.name, &self.tag)
    }
}
