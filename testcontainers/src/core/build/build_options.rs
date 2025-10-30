use std::collections::HashMap;

/// Options for configuring image build behavior.
///
/// Provides control over various aspects of the Docker image build process,
/// such as caching, whether to skip building if the image already exists, and build arguments.
///
/// # Example
///
/// ```rust,no_run
/// use testcontainers::{
///     core::BuildImageOptions,
///     runners::AsyncBuilder,
///     GenericBuildableImage,
/// };
///
/// # async fn example() -> anyhow::Result<()> {
/// let image = GenericBuildableImage::new("my-app", "latest")
///     .with_dockerfile_string("FROM alpine:latest\nARG VERSION\nRUN echo $VERSION")
///     .build_image_with(
///         BuildImageOptions::new()
///             .with_skip_if_exists(true)
///             .with_build_arg("VERSION", "1.0.0")
///     )
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BuildImageOptions {
    pub(crate) skip_if_exists: bool,
    pub(crate) no_cache: bool,
    pub(crate) build_args: HashMap<String, String>,
}

impl BuildImageOptions {
    /// Creates a new `BuildImageOptions` with default values.
    ///
    /// All options default to `false` and `build_args` is empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to skip building if the image already exists.
    ///
    /// When `true`, the build process will first check if an image with the
    /// specified descriptor (name:tag) already exists. If it does, the build
    /// is skipped and the existing image is used.
    ///
    /// Default: `false`
    pub fn with_skip_if_exists(mut self, skip_if_exists: bool) -> Self {
        self.skip_if_exists = skip_if_exists;
        self
    }

    /// Sets whether to disable build cache.
    ///
    /// When `true`, Docker will not use cached layers from previous builds,
    /// ensuring a completely fresh build from scratch.
    ///
    /// Default: `false`
    pub fn with_no_cache(mut self, no_cache: bool) -> Self {
        self.no_cache = no_cache;
        self
    }

    /// Adds a single build argument.
    ///
    /// Build arguments are passed to the Docker build process and can be used
    /// in the Dockerfile with `ARG` instructions. This method appends to existing
    /// build arguments, allowing multiple calls to add different arguments.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the build argument
    /// * `value` - The value of the build argument
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use testcontainers::core::BuildImageOptions;
    ///
    /// let opts = BuildImageOptions::new()
    ///     .with_build_arg("VERSION", "1.0.0")
    ///     .with_build_arg("BUILD_DATE", "2024-01-01");
    /// ```
    pub fn with_build_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.build_args.insert(key.into(), value.into());
        self
    }

    /// Replaces all build arguments with the provided HashMap.
    ///
    /// Build arguments are passed to the Docker build process and can be used
    /// in the Dockerfile with `ARG` instructions. This method replaces any
    /// existing build arguments.
    ///
    /// # Arguments
    ///
    /// * `build_args` - A HashMap of build argument names to values
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::collections::HashMap;
    /// use testcontainers::core::BuildImageOptions;
    ///
    /// let mut args = HashMap::new();
    /// args.insert("VERSION".to_string(), "1.0.0".to_string());
    /// args.insert("BUILD_DATE".to_string(), "2024-01-01".to_string());
    ///
    /// let opts = BuildImageOptions::new().with_build_args(args);
    /// ```
    pub fn with_build_args(mut self, build_args: HashMap<String, String>) -> Self {
        self.build_args = build_args;
        self
    }
}
