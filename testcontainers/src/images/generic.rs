use crate::{
    core::{ports::ContainerPort, WaitFor},
    Image,
};

/// A configurable image from which a [`Container`] or [`ContainerAsync`] can be started.
///
/// The various methods on this struct allow for configuring the resulting container using the
/// builder pattern. Further configuration is available through the [`ImageExt`] extension trait.
/// Make sure to invoke the configuration methods on [`GenericImage`] first, before those from
/// [`ImageExt`].
///
/// For example:
///
/// ```rust,ignore
/// use testcontainers::{
///     core::{IntoContainerPort, WaitFor}, runners::AsyncRunner, GenericImage, ImageExt
/// };
///
/// #[tokio::test]
/// async fn test_redis() {
///     let container = GenericImage::new("redis", "7.2.4")
///         .with_exposed_port(6379.tcp())
///         .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
///         .with_network("bridge")
///         .with_env_var("DEBUG", "1")
///         .start()
///         .await
///         .expect("Redis started");
/// #   container.stop().await.unwrap();
/// }
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # rt.block_on(test_redis());
/// ```
///
/// The extension traits [`SyncRunner`] and [`AsyncRunner`] each provide the method `start()` to
/// start the container once it is configured.
///
/// [`Container`]: crate::Container
/// [`ContainerAsync`]: crate::ContainerAsync
/// [`ImageExt`]: crate::core::ImageExt
/// [`SyncRunner`]: crate::runners::SyncRunner
/// [`AsyncRunner`]: crate::runners::AsyncRunner
#[must_use]
#[derive(Debug, Clone)]
pub struct GenericImage {
    name: String,
    tag: String,
    wait_for: Vec<WaitFor>,
    entrypoint: Option<String>,
    exposed_ports: Vec<ContainerPort>,
}

impl GenericImage {
    pub fn new<S: Into<String>>(name: S, tag: S) -> GenericImage {
        Self {
            name: name.into(),
            tag: tag.into(),
            wait_for: Vec::new(),
            entrypoint: None,
            exposed_ports: Vec::new(),
        }
    }

    pub fn with_wait_for(mut self, wait_for: WaitFor) -> Self {
        self.wait_for.push(wait_for);
        self
    }

    pub fn with_entrypoint(mut self, entrypoint: &str) -> Self {
        self.entrypoint = Some(entrypoint.to_string());
        self
    }

    pub fn with_exposed_port(mut self, port: ContainerPort) -> Self {
        self.exposed_ports.push(port);
        self
    }
}

impl Image for GenericImage {
    fn name(&self) -> &str {
        &self.name
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        self.wait_for.clone()
    }

    fn entrypoint(&self) -> Option<&str> {
        self.entrypoint.as_deref()
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &self.exposed_ports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImageExt;

    #[test]
    fn should_return_env_vars() {
        let image = GenericImage::new("hello-world", "latest")
            .with_env_var("one-key", "one-value")
            .with_env_var("two-key", "two-value");

        let mut env_vars = image.env_vars();
        let (first_key, first_value) = env_vars.next().unwrap();
        let (second_key, second_value) = env_vars.next().unwrap();

        assert_eq!(first_key, "one-key");
        assert_eq!(first_value, "one-value");
        assert_eq!(second_key, "two-key");
        assert_eq!(second_value, "two-value");
    }
}
