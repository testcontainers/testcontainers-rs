use std::{fmt, net::IpAddr, pin::Pin, str::FromStr, sync::Arc, time::Duration};

use tokio::io::{AsyncBufRead, AsyncReadExt};
use tokio_stream::StreamExt;

use crate::{
    core::{
        async_drop,
        client::Client,
        env,
        error::{ContainerMissingInfo, ExecError, Result, TestcontainersError},
        network::Network,
        ports::Ports,
        wait::WaitStrategy,
        CmdWaitFor, ContainerPort, ContainerState, ExecCommand, WaitFor,
    },
    ContainerRequest, Image,
};

pub(super) mod exec;

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
    id: String,
    image: ContainerRequest<I>,
    pub(super) docker_client: Arc<Client>,
    #[allow(dead_code)]
    network: Option<Arc<Network>>,
    dropped: bool,
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
        mut container_req: ContainerRequest<I>,
        network: Option<Arc<Network>>,
    ) -> Result<ContainerAsync<I>> {
        let log_consumers = std::mem::take(&mut container_req.log_consumers);
        let container = ContainerAsync {
            id,
            image: container_req,
            docker_client,
            network,
            dropped: false,
        };

        if !log_consumers.is_empty() {
            let mut logs = container.docker_client.logs(&container.id, true);
            let container_id = container.id.clone();
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

        let ready_conditions = container.image().ready_conditions();
        container.block_until_ready(ready_conditions).await?;
        Ok(container)
    }

    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a reference to the [`Image`] of this container.
    ///
    /// [`Image`]: trait.Image.html
    pub fn image(&self) -> &I {
        self.image.image()
    }

    pub async fn ports(&self) -> Result<Ports> {
        self.docker_client.ports(&self.id).await.map_err(Into::into)
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv4 interfaces.
    ///
    /// By default, `u16` is considered as TCP port. Also, you can convert `u16` to [`ContainerPort`] port
    /// by using [`crate::core::IntoContainerPort`] trait.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will return an error.
    pub async fn get_host_port_ipv4(&self, internal_port: impl Into<ContainerPort>) -> Result<u16> {
        let internal_port = internal_port.into();
        self.ports()
            .await?
            .map_to_host_port_ipv4(internal_port)
            .ok_or_else(|| TestcontainersError::PortNotExposed {
                id: self.id.clone(),
                port: internal_port,
            })
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv6 interfaces.
    ///
    /// By default, `u16` is considered as TCP port. Also, you can convert `u16` to [`ContainerPort`] port
    /// by using [`crate::core::IntoContainerPort`] trait.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will return an error.
    pub async fn get_host_port_ipv6(&self, internal_port: impl Into<ContainerPort>) -> Result<u16> {
        let internal_port = internal_port.into();
        self.ports()
            .await?
            .map_to_host_port_ipv6(internal_port)
            .ok_or_else(|| TestcontainersError::PortNotExposed {
                id: self.id.clone(),
                port: internal_port,
            })
    }

    /// Returns the bridge ip address of docker container as specified in NetworkSettings.Networks.IPAddress
    pub async fn get_bridge_ip_address(&self) -> Result<IpAddr> {
        let container_id = &self.id;
        let container_settings = self.docker_client.inspect(container_id).await?;

        let host_config = container_settings
            .host_config
            .ok_or_else(|| ContainerMissingInfo::new(container_id, "HostConfig"))?;

        let network_mode = host_config
            .network_mode
            .ok_or_else(|| ContainerMissingInfo::new(container_id, "HostConfig.NetworkMode"))?;

        let network_settings = self.docker_client.inspect_network(&network_mode).await?;

        network_settings.driver.ok_or_else(|| {
            TestcontainersError::other(format!("network {} is not in bridge mode", network_mode))
        })?;

        let container_network_settings = container_settings
            .network_settings
            .ok_or_else(|| ContainerMissingInfo::new(container_id, "NetworkSettings"))?;

        let mut networks = container_network_settings
            .networks
            .ok_or_else(|| ContainerMissingInfo::new(container_id, "NetworkSettings.Networks"))?;

        let ip = networks
            .remove(&network_mode)
            .and_then(|network| network.ip_address)
            .ok_or_else(|| {
                ContainerMissingInfo::new(container_id, "NetworkSettings.Networks.IpAddress")
            })?;

        IpAddr::from_str(&ip).map_err(TestcontainersError::other)
    }

    /// Returns the host that this container may be reached on (may not be the local machine)
    /// Suitable for use in URL
    pub async fn get_host(&self) -> Result<url::Host> {
        self.docker_client
            .docker_hostname()
            .await
            .map_err(Into::into)
    }

    /// Executes a command in the container.
    pub async fn exec(&self, cmd: ExecCommand) -> Result<exec::ExecResult> {
        let ExecCommand {
            cmd,
            container_ready_conditions,
            cmd_ready_condition,
        } = cmd;

        log::debug!("Executing command {:?}", cmd);

        let mut exec = self.docker_client.exec(&self.id, cmd).await?;
        self.block_until_ready(container_ready_conditions).await?;

        match cmd_ready_condition {
            CmdWaitFor::StdOutMessage { message } => {
                exec.stdout()
                    .wait_for_message(&message, 1)
                    .await
                    .map_err(ExecError::from)?;
            }
            CmdWaitFor::StdErrMessage { message } => {
                exec.stderr()
                    .wait_for_message(&message, 1)
                    .await
                    .map_err(ExecError::from)?;
            }
            CmdWaitFor::ExitCode { code } => {
                let exec_id = exec.id().to_string();
                loop {
                    let inspect = self.docker_client.inspect_exec(&exec_id).await?;

                    if let Some(actual) = inspect.exit_code {
                        if actual != code {
                            Err(ExecError::ExitCodeMismatch {
                                expected: code,
                                actual,
                            })?;
                        }
                        break;
                    } else {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
            CmdWaitFor::Duration { length } => {
                tokio::time::sleep(length).await;
            }
            _ => {}
        }

        Ok(exec::ExecResult {
            client: self.docker_client.clone(),
            id: exec.id,
            stdout: exec.stdout.into_inner(),
            stderr: exec.stderr.into_inner(),
        })
    }

    /// Starts the container.
    pub async fn start(&self) -> Result<()> {
        self.docker_client.start(&self.id).await?;
        let state = ContainerState::new(self.id(), self.ports().await?);
        for cmd in self.image.exec_after_start(state)? {
            self.exec(cmd).await?;
        }
        Ok(())
    }

    /// Stops the container (not the same with `pause`).
    pub async fn stop(&self) -> Result<()> {
        log::debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id).await?;
        Ok(())
    }

    /// Removes the container.
    pub async fn rm(mut self) -> Result<()> {
        log::debug!("Deleting docker container {}", self.id);

        self.docker_client.rm(&self.id).await?;

        #[cfg(feature = "watchdog")]
        crate::watchdog::unregister(&self.id);

        self.dropped = true;
        Ok(())
    }

    /// Returns an asynchronous reader for stdout.
    ///
    /// Accepts a boolean parameter to follow the logs:
    ///   - pass `true` to read logs from the moment the container starts until it stops (returns I/O error with kind [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) if container removed).
    ///   - pass `false` to read logs from startup to present.
    pub fn stdout(&self, follow: bool) -> Pin<Box<dyn AsyncBufRead + Send>> {
        let stdout = self.docker_client.stdout_logs(&self.id, follow);
        Box::pin(tokio_util::io::StreamReader::new(stdout))
    }

    /// Returns an asynchronous reader for stderr.
    ///
    /// Accepts a boolean parameter to follow the logs:
    ///   - pass `true` to read logs from the moment the container starts until it stops (returns I/O error with [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof) if container removed).
    ///   - pass `false` to read logs from startup to present.
    pub fn stderr(&self, follow: bool) -> Pin<Box<dyn AsyncBufRead + Send>> {
        let stderr = self.docker_client.stderr_logs(&self.id, follow);
        Box::pin(tokio_util::io::StreamReader::new(stderr))
    }

    /// Returns stdout as a vector of bytes available at the moment of call (from container startup to present).
    ///
    /// If you want to read stdout in asynchronous manner, use [`ContainerAsync::stdout`] instead.
    pub async fn stdout_to_vec(&self) -> Result<Vec<u8>> {
        let mut stdout = Vec::new();
        self.stdout(false).read_to_end(&mut stdout).await?;
        Ok(stdout)
    }

    /// Returns stderr as a vector of bytes available at the moment of call (from container startup to present).
    ///
    /// If you want to read stderr in asynchronous manner, use [`ContainerAsync::stderr`] instead.
    pub async fn stderr_to_vec(&self) -> Result<Vec<u8>> {
        let mut stderr = Vec::new();
        self.stderr(false).read_to_end(&mut stderr).await?;
        Ok(stderr)
    }

    pub(crate) async fn block_until_ready(&self, ready_conditions: Vec<WaitFor>) -> Result<()> {
        log::debug!("Waiting for container {} to be ready", self.id);
        let id = self.id();

        for condition in ready_conditions {
            condition
                .wait_until_ready(&self.docker_client, self)
                .await?;
        }

        log::debug!("Container {id} is now ready!");
        Ok(())
    }
}

impl<I> fmt::Debug for ContainerAsync<I>
where
    I: fmt::Debug + Image,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContainerAsync")
            .field("id", &self.id)
            .field("image", &self.image)
            .field("command", &self.docker_client.config.command())
            .finish()
    }
}

impl<I> Drop for ContainerAsync<I>
where
    I: Image,
{
    fn drop(&mut self) {
        if !self.dropped {
            let id = self.id.clone();
            let client = self.docker_client.clone();
            let command = self.docker_client.config.command();

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
            // async_drop::block_on!(drop_task, "failed to remove container on drop");
        }
    }
}

#[cfg(test)]
mod tests {

    use tokio::io::AsyncBufReadExt;

    use crate::{images::generic::GenericImage, runners::AsyncRunner};

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
}
