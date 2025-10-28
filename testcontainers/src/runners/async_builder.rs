use async_trait::async_trait;

use crate::{
    core::{build::build_options::BuildImageOptions, client::Client, error::Result},
    BuildableImage,
};

#[async_trait]
pub trait AsyncBuilder<B: BuildableImage> {
    async fn build_image(self) -> Result<B::Built>;
    async fn build_image_with(self, options: BuildImageOptions) -> Result<B::Built>;
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
        self.build_image_with(BuildImageOptions::default()).await
    }

    async fn build_image_with(self, options: BuildImageOptions) -> Result<T::Built> {
        let client = Client::lazy_client().await?;

        let build_context = self.build_context();
        let descriptor = self.descriptor();

        client
            .build_image(&descriptor, &build_context, options)
            .await?;

        Ok(self.into_image())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{BuildImageOptions, WaitFor},
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

    #[tokio::test]
    async fn build_image_with_options() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let image = GenericBuildableImage::new("hello-tc-with-options", "test")
            .with_dockerfile_string(
                r#"FROM alpine:latest
COPY --chmod=0755 hello.sh /sbin/hello
ENTRYPOINT ["/sbin/hello"]
"#,
            )
            .with_data(
                r#"#!/bin/sh
echo "hello from build_image_with_options""#,
                "./hello.sh",
            )
            .build_image_with(BuildImageOptions::new().with_no_cache(true))
            .await?;

        let _container = image
            .with_wait_for(WaitFor::message_on_stdout(
                "hello from build_image_with_options",
            ))
            .start()
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn build_image_skip_if_exists() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let image1 = GenericBuildableImage::new("hello-tc-skip", "test")
            .with_dockerfile_string(
                r#"FROM alpine:latest
COPY --chmod=0755 hello.sh /sbin/hello
ENTRYPOINT ["/sbin/hello"]
"#,
            )
            .with_data(
                r#"#!/bin/sh
echo "hello from skip test""#,
                "./hello.sh",
            )
            .build_image_with(BuildImageOptions::new())
            .await?;

        let _container1 = image1
            .with_wait_for(WaitFor::message_on_stdout("hello from skip test"))
            .start()
            .await?;

        let image2 = GenericBuildableImage::new("hello-tc-skip", "test")
            .with_dockerfile_string(
                r#"FROM alpine:latest
COPY --chmod=0755 hello.sh /sbin/hello
ENTRYPOINT ["/sbin/hello"]
"#,
            )
            .with_data(
                r#"#!/bin/sh
echo "hello from skip test""#,
                "./hello.sh",
            )
            .build_image_with(BuildImageOptions::new().with_skip_if_exists(true))
            .await?;

        let _container2 = image2
            .with_wait_for(WaitFor::message_on_stdout("hello from skip test"))
            .start()
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn build_image_parallel_with_skip_if_exists() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let build_task = || async {
            GenericBuildableImage::new("hello-tc-parallel", "test")
                .with_dockerfile_string(
                    r#"FROM alpine:latest
COPY --chmod=0755 hello.sh /sbin/hello
ENTRYPOINT ["/sbin/hello"]
"#,
                )
                .with_data(
                    r#"#!/bin/sh
echo "hello from parallel test""#,
                    "./hello.sh",
                )
                .build_image_with(BuildImageOptions::new().with_skip_if_exists(true))
                .await
        };

        let (result1, result2, result3) = tokio::join!(build_task(), build_task(), build_task());

        let image1 = result1?;
        let image2 = result2?;
        let image3 = result3?;

        let _container1 = image1
            .with_wait_for(WaitFor::message_on_stdout("hello from parallel test"))
            .start()
            .await?;

        let _container2 = image2
            .with_wait_for(WaitFor::message_on_stdout("hello from parallel test"))
            .start()
            .await?;

        let _container3 = image3
            .with_wait_for(WaitFor::message_on_stdout("hello from parallel test"))
            .start()
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn build_image_with_build_args() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let image = GenericBuildableImage::new("hello-tc-buildargs", "test")
            .with_dockerfile_string(
                r#"FROM alpine:latest
ARG VERSION=unknown
ARG BUILD_DATE=unknown
RUN echo "Building with VERSION=${VERSION} DATE=${BUILD_DATE}" > /build_info.txt
COPY --chmod=0755 hello.sh /sbin/hello
ENTRYPOINT ["/sbin/hello"]
"#,
            )
            .with_data(
                r#"#!/bin/sh
cat /build_info.txt"#,
                "./hello.sh",
            )
            .build_image_with(
                BuildImageOptions::new()
                    .with_build_arg("VERSION", "1.0.0")
                    .with_build_arg("BUILD_DATE", "2024-10-25"),
            )
            .await?;

        let _container = image
            .with_wait_for(WaitFor::message_on_stdout("VERSION=1.0.0"))
            .start()
            .await?;

        Ok(())
    }
}
