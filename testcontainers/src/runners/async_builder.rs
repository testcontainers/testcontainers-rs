use async_trait::async_trait;

use crate::{
    core::{client::Client, error::Result},
    BuildableImage,
};

#[async_trait]
pub trait AsyncBuilder<B: BuildableImage> {
    async fn build_image(self) -> Result<B::Built>;
}

#[async_trait]
/// Helper trait to build Docker images asynchronously from [`BuildableImage`] instances.
///
/// Provides an asynchronous interface for building custom Docker images within test environments.
/// This trait is automatically implemented for any type that implements [`BuildableImage`] + [`Send`].
///
/// # Example
///
/// ```rust,no_run
/// use testcontainers::{core::WaitFor, runners::AsyncBuilder, runners::AsyncRunner, GenericBuildableImage};
///
/// #[test]
/// async fn test_custom_image() -> anyhow::Result<()> {
///     let image = GenericBuildableImage::new("my-test-app", "latest")
///         .with_dockerfile_string("FROM alpine:latest\nRUN echo 'hello'")
///         .build_image()?.await;
///     // Use the built image in containers
///     let container = image
///         .with_wait_for(WaitFor::message_on_stdout("Hello from test!"))
///         .start()?.await;
///
///     Ok(())
/// }
/// ```
impl<T> AsyncBuilder<T> for T
where
    T: BuildableImage + Send,
{
    async fn build_image(self) -> Result<T::Built> {
        let client = Client::lazy_client().await?;

        // Get build context and image descriptor from the buildable image
        let build_context = self.build_context();
        let descriptor = self.descriptor();

        client.build_image(&descriptor, &build_context).await?;

        // consume the BuildableImage into an Image for running
        Ok(self.into_image())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::WaitFor,
        runners::{AsyncBuilder, AsyncRunner},
        GenericBuildableImage,
    };

    #[tokio::test]
    async fn build_image_and_run() -> anyhow::Result<()> {
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
            .build_image()
            .await?;

        let _container = image
            .with_wait_for(WaitFor::message_on_stdout("hello from hello-tc"))
            .start()
            .await?;

        Ok(())
    }
}
