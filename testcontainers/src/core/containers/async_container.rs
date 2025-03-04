use std::{fmt, ops::Deref, sync::Arc};

use tokio_stream::StreamExt;

use crate::{
    core::{async_drop, client::Client, env, error::Result, network::Network, ContainerState},
    ContainerRequest, Image,
};

pub(super) mod exec;
pub(crate) mod raw;

/// Represents a running docker container that has been started using an async client.
///
/// Containers have a [`custom destructor`][drop_impl] that removes them as soon as they
/// go out of scope. However, async drop is not available in rust yet. This implementation
/// is using block_on.
///
/// ```rust,no_run
/// use testcontainers::*;
/// #[tokio::test]
/// async fn a_test() {
///     let container = MyImage::default().start().await.unwrap();
///     // Docker container is stopped/removed at the end of this scope.
/// }
/// ```
///
/// [drop_impl]: struct.ContainerAsync.html#impl-Drop
pub struct ContainerAsync<I: Image> {
    pub(super) raw: raw::RawContainer,
    image: ContainerRequest<I>,
    network: Option<Arc<Network>>,
    dropped: bool,
    #[cfg(feature = "reusable-containers")]
    reuse: crate::ReuseDirective,
}

impl<I> ContainerAsync<I>
where
    I: Image,
{
    /// Constructs a new container given an id, a docker client and the image.
    /// ContainerAsync::new().await
    pub(crate) async fn new(
        id: String,
        docker_client: Arc<Client>,
        container_req: ContainerRequest<I>,
        network: Option<Arc<Network>>,
    ) -> Result<ContainerAsync<I>> {
        let container = Self::construct(id, docker_client, container_req, network);
        let state = ContainerState::from_container(&container).await?;
        for cmd in container.image().exec_before_ready(state)? {
            container.exec(cmd).await?;
        }
        let ready_conditions = container.image().ready_conditions();
        container.block_until_ready(ready_conditions).await?;
        Ok(container)
    }

    pub(crate) fn construct(
        id: String,
        docker_client: Arc<Client>,
        mut container_req: ContainerRequest<I>,
        network: Option<Arc<Network>>,
    ) -> ContainerAsync<I> {
        #[cfg(feature = "reusable-containers")]
        let reuse = container_req.reuse();

        let log_consumers = std::mem::take(&mut container_req.log_consumers);
        let container = ContainerAsync {
            raw: raw::RawContainer::new(id, docker_client),
            image: container_req,
            network,
            dropped: false,
            #[cfg(feature = "reusable-containers")]
            reuse,
        };

        if !log_consumers.is_empty() {
            let mut logs = container.docker_client().logs(container.id(), true);
            let container_id = container.id().to_string();
            tokio::spawn(async move {
                while let Some(result) = logs.next().await {
                    match result {
                        Ok(record) => {
                            for consumer in &log_consumers {
                                consumer.accept(&record).await;
                                tokio::task::yield_now().await;
                            }
                        }
                        Err(err) => {
                            log::warn!(
                                "Failed to read log frame for container {container_id}: {err}",
                            );
                        }
                    }
                }
            });
        }

        container
    }

    /// Returns a reference to the [`Image`] of this container.
    ///
    /// [`Image`]: trait.Image.html
    pub fn image(&self) -> &I {
        self.image.image()
    }

    /// Starts the container.
    pub async fn start(&self) -> Result<()> {
        self.raw.start().await?;
        let state = ContainerState::from_container(self).await?;
        for cmd in self.image.exec_after_start(state)? {
            self.raw.exec(cmd).await?;
        }
        Ok(())
    }

    /// Removes the container.
    pub async fn rm(mut self) -> Result<()> {
        log::debug!("Deleting docker container {}", self.id());

        self.raw.docker_client().rm(self.id()).await?;

        #[cfg(feature = "watchdog")]
        crate::watchdog::unregister(self.id());

        self.dropped = true;
        Ok(())
    }
}

impl<I> fmt::Debug for ContainerAsync<I>
where
    I: fmt::Debug + Image,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut repr = f.debug_struct("ContainerAsync");

        repr.field("id", &self.id())
            .field("image", &self.image)
            .field("command", &self.docker_client().config.command())
            .field("network", &self.network)
            .field("dropped", &self.dropped);

        #[cfg(feature = "reusable-containers")]
        repr.field("reuse", &self.reuse);

        repr.finish()
    }
}

impl<I> Drop for ContainerAsync<I>
where
    I: Image,
{
    fn drop(&mut self) {
        #[cfg(feature = "reusable-containers")]
        {
            use crate::ReuseDirective::{Always, CurrentSession};

            if !self.dropped && matches!(self.reuse, Always | CurrentSession) {
                log::debug!(
                    "Declining to reap container marked for reuse: {}",
                    &self.id()
                );

                return;
            }
        }

        if !self.dropped {
            let id = self.id().to_string();
            let client = self.docker_client().clone();
            let command = self.docker_client().config.command();

            let drop_task = async move {
                log::trace!("Drop was called for container {id}, cleaning up");
                match command {
                    env::Command::Remove => {
                        if let Err(e) = client.rm(&id).await {
                            log::error!("Failed to remove container on drop: {}", e);
                        }
                    }
                    env::Command::Keep => {}
                }
                #[cfg(feature = "watchdog")]
                crate::watchdog::unregister(&id);

                log::debug!("Container {id} was successfully dropped");
            };

            async_drop::async_drop(drop_task);
        }
    }
}

impl<I: Image> Deref for ContainerAsync<I> {
    type Target = raw::RawContainer;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncBufReadExt;

    use crate::{
        core::{ContainerPort, ContainerState, ExecCommand, WaitFor},
        images::generic::GenericImage,
        runners::AsyncRunner,
        Image,
    };

    #[tokio::test]
    async fn async_logs_are_accessible() -> anyhow::Result<()> {
        let image = GenericImage::new("testcontainers/helloworld", "1.1.0");
        let container = image.start().await?;

        let stderr = container.stderr(true);

        // it's possible to send logs into background task
        let log_follower_task = tokio::spawn(async move {
            let mut stderr_lines = stderr.lines();
            let expected_messages = [
                "DELAY_START_MSEC: 0",
                "Sleeping for 0 ms",
                "Starting server on port 8080",
                "Sleeping for 0 ms",
                "Starting server on port 8081",
                "Ready, listening on 8080 and 8081",
            ];
            for expected_message in expected_messages {
                let line = stderr_lines.next_line().await?.expect("line must exist");
                if !line.contains(expected_message) {
                    anyhow::bail!(
                        "Log message ('{}') doesn't contain expected message ('{}')",
                        line,
                        expected_message
                    );
                }
            }
            Ok(())
        });
        log_follower_task
            .await
            .map_err(|_| anyhow::anyhow!("failed to join log follower task"))??;

        // logs are accessible after container is stopped
        container.stop().await?;

        // stdout is empty
        let stdout = String::from_utf8(container.stdout_to_vec().await?)?;
        assert_eq!(stdout, "");
        // stderr contains 6 lines
        let stderr = String::from_utf8(container.stderr_to_vec().await?)?;
        assert_eq!(
            stderr.lines().count(),
            6,
            "unexpected stderr size: {stderr}",
        );

        // start again to test eof on drop
        container.start().await?;

        // create logger task which reads logs from container up to EOF
        let container_id = container.id().to_string();
        let stderr = container.stderr(true);
        let logger_task = tokio::spawn(async move {
            let mut stderr_lines = stderr.lines();
            while let Some(result) = stderr_lines.next_line().await.transpose() {
                match result {
                    Ok(line) => {
                        log::debug!(target: "container", "[{container_id}]:{}", line);
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                        log::debug!(target: "container", "[{container_id}] EOF");
                        break;
                    }
                    Err(err) => Err(err)?,
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        drop(container);
        let res = logger_task
            .await
            .map_err(|_| anyhow::anyhow!("failed to join log follower task"))?;
        assert!(
            res.is_ok(),
            "UnexpectedEof is handled after dropping the container"
        );

        Ok(())
    }

    #[cfg(feature = "reusable-containers")]
    #[tokio::test]
    async fn async_containers_are_reused() -> anyhow::Result<()> {
        use crate::ImageExt;

        let labels = [
            ("foo", "bar"),
            ("baz", "qux"),
            ("test-name", "async_containers_are_reused"),
        ];

        let initial_image = GenericImage::new("testcontainers/helloworld", "1.1.0")
            .with_reuse(crate::ReuseDirective::CurrentSession)
            .with_labels(labels);

        let reused_image = initial_image
            .image
            .clone()
            .with_reuse(crate::ReuseDirective::CurrentSession)
            .with_labels(labels);

        let initial_container = initial_image.start().await?;
        let reused_container = reused_image.start().await?;

        assert_eq!(initial_container.id(), reused_container.id());

        let client = crate::core::client::docker_client_instance().await?;

        let options = bollard::container::ListContainersOptions {
            all: false,
            limit: Some(2),
            size: false,
            filters: std::collections::HashMap::from_iter([(
                "label".to_string(),
                labels
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .chain([
                        "org.testcontainers.managed-by=testcontainers".to_string(),
                        format!(
                            "org.testcontainers.session-id={}",
                            crate::runners::async_runner::session_id()
                        ),
                    ])
                    .collect(),
            )]),
        };

        let containers = client.list_containers(Some(options)).await?;

        assert_eq!(containers.len(), 1);

        assert_eq!(
            Some(initial_container.id()),
            containers.first().unwrap().id.as_deref()
        );

        reused_container.rm().await.map_err(anyhow::Error::from)
    }

    #[cfg(feature = "reusable-containers")]
    #[tokio::test]
    async fn async_reused_containers_are_not_confused() -> anyhow::Result<()> {
        use std::collections::HashSet;

        use crate::{ImageExt, ReuseDirective};

        let labels = [
            ("foo", "bar"),
            ("baz", "qux"),
            ("test-name", "async_reused_containers_are_not_confused"),
        ];

        let initial_image = GenericImage::new("testcontainers/helloworld", "1.1.0")
            .with_reuse(ReuseDirective::Always)
            .with_labels(labels);

        let similar_image = initial_image
            .image
            .clone()
            .with_reuse(ReuseDirective::Never)
            .with_labels(&initial_image.labels);

        let initial_container = initial_image.start().await?;
        let similar_container = similar_image.start().await?;

        assert_ne!(initial_container.id(), similar_container.id());

        let client = crate::core::client::docker_client_instance().await?;

        let options = bollard::container::ListContainersOptions {
            all: false,
            limit: Some(2),
            size: false,
            filters: std::collections::HashMap::from_iter([(
                "label".to_string(),
                labels
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .chain(["org.testcontainers.managed-by=testcontainers".to_string()])
                    .collect(),
            )]),
        };

        let containers = client.list_containers(Some(options)).await?;

        assert_eq!(containers.len(), 2);

        let container_ids = containers
            .iter()
            .filter_map(|container| container.id.as_deref())
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(
            container_ids,
            HashSet::from_iter([initial_container.id(), similar_container.id()])
        );

        initial_container.rm().await?;
        similar_container.rm().await.map_err(anyhow::Error::from)
    }

    #[cfg(feature = "reusable-containers")]
    #[tokio::test]
    async fn async_reusable_containers_are_not_dropped() -> anyhow::Result<()> {
        use crate::{ImageExt, ReuseDirective};

        let client = crate::core::client::docker_client_instance().await?;

        let image = GenericImage::new("testcontainers/helloworld", "1.1.0")
            .with_reuse(ReuseDirective::Always)
            .with_labels([
                ("foo", "bar"),
                ("baz", "qux"),
                ("test-name", "async_reusable_containers_are_not_dropped"),
            ]);

        let container_id = {
            let container = image.start().await?;

            assert!(!container.dropped);
            assert_eq!(container.reuse, ReuseDirective::Always);

            container.id().to_string()
        };

        assert!(client
            .inspect_container(&container_id, None)
            .await?
            .state
            .and_then(|state| state.running)
            .unwrap_or(false));

        client
            .remove_container(
                &container_id,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(anyhow::Error::from)
    }

    #[cfg(feature = "http_wait")]
    #[tokio::test]
    async fn exec_before_ready_is_ran() {
        use crate::core::wait::HttpWaitStrategy;

        struct ExecBeforeReady {}

        impl Image for ExecBeforeReady {
            fn name(&self) -> &str {
                "testcontainers/helloworld"
            }

            fn tag(&self) -> &str {
                "1.2.0"
            }

            fn ready_conditions(&self) -> Vec<WaitFor> {
                vec![WaitFor::http(
                    HttpWaitStrategy::new("/ping")
                        .with_port(ContainerPort::Tcp(8080))
                        .with_expected_status_code(200u16),
                )]
            }

            fn expose_ports(&self) -> &[ContainerPort] {
                &[ContainerPort::Tcp(8080)]
            }

            #[allow(unused)]
            fn exec_before_ready(
                &self,
                cs: ContainerState,
            ) -> crate::core::error::Result<Vec<ExecCommand>> {
                Ok(vec![ExecCommand::new(vec![
                    "/bin/sh",
                    "-c",
                    "echo 'exec_before_ready ran!' > /opt/hello",
                ])])
            }
        }

        let container = ExecBeforeReady {};
        let container = container.start().await.unwrap();
        let mut exec_result = container
            .exec(ExecCommand::new(vec!["cat", "/opt/hello"]))
            .await
            .unwrap();
        let stdout = exec_result.stdout_to_vec().await.unwrap();
        let output = String::from_utf8(stdout).unwrap();
        assert_eq!(output, "exec_before_ready ran!\n");
    }
}
