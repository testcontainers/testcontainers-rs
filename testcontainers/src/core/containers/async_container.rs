use core::panic;
use std::{fmt, net::IpAddr, str::FromStr, sync::Arc};

use tokio::runtime::RuntimeFlavor;

use crate::{
    core::{
        client::{AttachLog, Client},
        env,
        image::CmdWaitFor,
        macros,
        network::Network,
        ports::Ports,
        ContainerState, ExecCommand,
    },
    Image, RunnableImage,
};

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
///     let container = MyImage::default().start().await;
///     // Docker container is stopped/removed at the end of this scope.
/// }
/// ```
///
/// [drop_impl]: struct.ContainerAsync.html#impl-Drop
pub struct ContainerAsync<I: Image> {
    id: String,
    image: RunnableImage<I>,
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
        image: RunnableImage<I>,
        network: Option<Arc<Network>>,
    ) -> ContainerAsync<I> {
        let container = ContainerAsync {
            id,
            image,
            docker_client,
            network,
            dropped: false,
        };
        container.block_until_ready().await;
        container
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

    /// Returns a reference to the [`arguments`] of the [`Image`] of this container.
    ///
    /// Access to this is useful to retrieve relevant information which had been passed as [`arguments`]
    ///
    /// [`Image`]: trait.Image.html
    /// [`arguments`]: trait.Image.html#associatedtype.Args
    pub fn image_args(&self) -> &I::Args {
        self.image.args()
    }

    pub async fn ports(&self) -> Ports {
        self.docker_client.ports(&self.id).await
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv4 interfaces.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will panic.
    ///
    /// # Panics
    ///
    /// This method panics if the given port is not mapped.
    /// Testcontainers is designed to be used in tests only. If a certain port is not mapped, the container
    /// is unlikely to be useful.
    pub async fn get_host_port_ipv4(&self, internal_port: u16) -> u16 {
        self.docker_client
            .ports(&self.id)
            .await
            .map_to_host_port_ipv4(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "container {} does not expose (IPV4) port {}",
                    self.id, internal_port
                )
            })
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv6 interfaces.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will panic.
    ///
    /// # Panics
    ///
    /// This method panics if the given port is not mapped.
    /// Testcontainers is designed to be used in tests only. If a certain port is not mapped, the container
    /// is unlikely to be useful.
    pub async fn get_host_port_ipv6(&self, internal_port: u16) -> u16 {
        self.docker_client
            .ports(&self.id)
            .await
            .map_to_host_port_ipv6(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "container {} does not expose (IPV6) port {}",
                    self.id, internal_port
                )
            })
    }

    /// Returns the bridge ip address of docker container as specified in NetworkSettings.Networks.IPAddress
    pub async fn get_bridge_ip_address(&self) -> IpAddr {
        let container_settings = self.docker_client.inspect(&self.id).await;

        let host_config = container_settings
            .host_config
            .unwrap_or_else(|| panic!("container {} has no host config settings", self.id));

        let network_mode = host_config
            .network_mode
            .unwrap_or_else(|| panic!("container {} has no network mode", self.id));

        let network_settings = self
            .docker_client
            .inspect_network(&network_mode)
            .await
            .unwrap_or_else(|_| panic!("container {} network mode does not exist", self.id));

        network_settings
            .driver
            .unwrap_or_else(|| panic!("network {} is not in bridge mode", network_mode));

        let container_network_settings = container_settings
            .network_settings
            .unwrap_or_else(|| panic!("container {} has no network settings", self.id));

        let mut networks = container_network_settings
            .networks
            .unwrap_or_else(|| panic!("container {} has no networks", self.id));

        let ip = networks
            .remove(&network_mode)
            .and_then(|network| network.ip_address)
            .unwrap_or_else(|| panic!("container {} has missing bridge IP", self.id));

        IpAddr::from_str(&ip)
            .unwrap_or_else(|_| panic!("container {} has invalid bridge IP", self.id))
    }

    /// Returns the host ip address of docker container
    #[deprecated(since = "0.16.6", note = "Please use `get_host` instead")]
    pub async fn get_host_ip_address(&self) -> IpAddr {
        let host_name = self.docker_client.docker_hostname().await;

        match host_name {
            // todo: drop `dns_lookup` dependency after dropping this method
            url::Host::Domain(domain) => dns_lookup::lookup_host(&domain)
                .ok()
                .and_then(|ips| ips.into_iter().next())
                .unwrap_or(IpAddr::from([127, 0, 0, 1])),
            url::Host::Ipv4(ip) => ip.into(),
            url::Host::Ipv6(ip) => ip.into(),
        }
    }

    /// Returns the host that this container may be reached on (may not be the local machine)
    /// Suitable for use in URL
    pub async fn get_host(&self) -> url::Host {
        self.docker_client.docker_hostname().await
    }

    pub async fn exec(&self, cmd: ExecCommand) {
        let ExecCommand {
            cmd,
            container_ready_conditions,
            cmd_ready_condition,
        } = cmd;

        log::debug!("Executing command {:?}", cmd);

        let attach_log = match cmd_ready_condition {
            CmdWaitFor::StdOutMessage { .. } => AttachLog::stdout(),
            CmdWaitFor::StdErrMessage { .. } => AttachLog::stderr(),
            CmdWaitFor::StdOutOrErrMessage { .. } => AttachLog::stdout_and_stderr(),
            _ => AttachLog::nothing(),
        };

        let (exec_id, output) = self.docker_client.exec(&self.id, cmd, attach_log).await;
        self.docker_client
            .block_until_ready(self.id(), &container_ready_conditions)
            .await;

        match cmd_ready_condition {
            CmdWaitFor::StdOutOrErrMessage { message }
            | CmdWaitFor::StdOutMessage { message }
            | CmdWaitFor::StdErrMessage { message } => {
                output.wait_for_message(&message).await.unwrap();
            }
            CmdWaitFor::ExitCode { code } => loop {
                let inspect = self
                    .docker_client
                    .bollard
                    .inspect_exec(&exec_id)
                    .await
                    .unwrap();

                if let Some(exit_code) = inspect.exit_code {
                    assert_eq!(
                        exit_code, code,
                        "expected exit code {} but got {:?}",
                        code, inspect.exit_code
                    );
                    break;
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            },
            CmdWaitFor::Duration { length } => {
                tokio::time::sleep(length).await;
            }
            _ => {}
        }
    }

    pub async fn start(&self) {
        self.docker_client.start(&self.id).await;
        for cmd in self
            .image
            .exec_after_start(ContainerState::new(self.ports().await))
        {
            self.exec(cmd).await;
        }
    }

    pub async fn stop(&self) {
        log::debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id).await
    }

    pub async fn rm(mut self) {
        log::debug!("Deleting docker container {}", self.id);

        self.docker_client.rm(&self.id).await;

        #[cfg(feature = "watchdog")]
        crate::watchdog::unregister(&self.id);

        self.dropped = true;
    }

    async fn block_until_ready(&self) {
        self.docker_client
            .block_until_ready(self.id(), &self.image().ready_conditions())
            .await;
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
                    env::Command::Remove => client.rm(&id).await,
                    env::Command::Keep => {}
                }
                #[cfg(feature = "watchdog")]
                crate::watchdog::unregister(&id);

                log::debug!("Container {id} was successfully dropped");
            };

            macros::block_on!(drop_task, "failed to remove container on drop");
        }
    }
}
