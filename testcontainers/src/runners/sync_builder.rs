use crate::{core::error::Result, runners::sync_runner::lazy_sync_runner, BuildableImage};

/// Helper trait to build Docker images synchronously from [`BuildableImage`] instances.
///
/// Provides a blocking interface for building custom Docker images within test environments.
/// This trait is automatically implemented for any type that implements [`BuildableImage`] + [`Send`].
///
/// # Example
///
/// ```rust,no_run
/// use testcontainers::{core::WaitFor, runners::SyncBuilder, runners::SyncRunner, GenericBuildableImage};
///
/// #[test]
/// fn test_custom_image() -> anyhow::Result<()> {
///     let image = GenericBuildableImage::new("my-test-app", "latest")
///         .with_dockerfile_string("FROM alpine:latest\nRUN echo 'hello'")
///         .build_image()?;
///     // Use the built image in containers
///     let container = image
///         .with_wait_for(WaitFor::message_on_stdout("Hello from test!"))
///         .start()?;
///
///     Ok(())
/// }
/// ```
pub trait SyncBuilder<B: BuildableImage> {
    fn build_image(self) -> Result<B::Built>;
}

impl<T> SyncBuilder<T> for T
where
    T: BuildableImage + Send,
{
    fn build_image(self) -> Result<T::Built> {
        let runtime = lazy_sync_runner()?;
        runtime.block_on(super::AsyncBuilder::build_image(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::WaitFor,
        runners::{SyncBuilder, SyncRunner},
        GenericBuildableImage,
    };

    #[test]
    fn build_image_and_run() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let image = GenericBuildableImage::new("hello-tc", "latest")
            .with_dockerfile_string(
                r#"FROM alpine:latest
COPY --chmod=0755 hello.sh /sbin/hello
ENTRYPOINT ["/sbin/hello"]
"#,
            )
            .with_data(
                r#"#!/bin/sh
echo "hello from hello-tc""#,
                "./hello.sh",
            )
            .build_image()?;

        let _container = image
            .with_wait_for(WaitFor::message_on_stdout("hello from hello-tc"))
            .start()?;

        Ok(())
    }
}
