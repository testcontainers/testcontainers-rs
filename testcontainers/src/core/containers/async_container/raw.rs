use std::{fmt, net::IpAddr, pin::Pin, str::FromStr, sync::Arc, time::Duration};

use tokio::io::{AsyncBufRead, AsyncReadExt};

use super::{exec, Client};
use crate::{
    core::{
        copy::CopyFileFromContainer,
        error::{ContainerMissingInfo, ExecError, Result},
        ports::Ports,
        wait::WaitStrategy,
        CmdWaitFor, ContainerPort, ExecCommand, WaitFor,
    },
    TestcontainersError,
};

/// Represents a docker container without any additional functionality.
/// It basically wraps a docker container id and a docker client to expose some common functionality.
pub struct RawContainer {
    id: String,
    docker_client: Arc<Client>,
}

impl RawContainer {
    pub(crate) fn new(id: String, docker_client: Arc<Client>) -> Self {
        Self { id, docker_client }
    }

    pub(crate) fn docker_client(&self) -> &Arc<Client> {
        &self.docker_client
    }

    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn ports(&self) -> Result<Ports> {
        self.docker_client.ports(&self.id).await.map_err(Into::into)
    }

    pub(crate) async fn copy_file_from<T>(
        &self,
        container_path: impl Into<String>,
        target: T,
    ) -> Result<T::Output>
    where
        T: CopyFileFromContainer,
    {
        let container_path = container_path.into();
        self.docker_client
            .copy_file_from_container(self.id(), &container_path, target)
            .await
            .map_err(TestcontainersError::from)
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
            CmdWaitFor::Exit { code } => {
                let exec_id = exec.id().to_string();
                loop {
                    let inspect = self.docker_client.inspect_exec(&exec_id).await?;

                    if let Some(actual) = inspect.exit_code {
                        if let Some(expected) = code {
                            if actual != expected {
                                Err(ExecError::ExitCodeMismatch { expected, actual })?;
                            }
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
        Ok(())
    }

    /// Stops the container (not the same with `pause`) using the default timeout.
    pub async fn stop(&self) -> Result<()> {
        log::debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id, None).await?;
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

impl fmt::Debug for RawContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut repr = f.debug_struct("ContainerAsync");

        repr.field("id", &self.id)
            .field("command", &self.docker_client.config.command());

        repr.finish()
    }
}
