use crate::{
    core::{env::Command, logs::LogStream, ports::Ports, ExecCommand, WaitFor},
    Image, RunnableImage,
};
use bollard_stubs::models::ContainerInspectResponse;

use std::{fmt, net::IpAddr, str::FromStr};

/// Represents a running docker container.
///
/// Containers have a [`custom destructor`][drop_impl] that removes them as soon as they go out of scope:
///
/// ```rust
/// use testcontainers::*;
/// #[test]
/// fn a_test() {
///     let docker = clients::Cli::default();
///
///     {
///         let container = docker.run(MyImage::default());
///
///         // Docker container is stopped/removed at the end of this scope.
///     }
/// }
///
/// ```
///
/// [drop_impl]: struct.Container.html#impl-Drop
pub struct Container<I: Image> {
    id: String,
    docker_client: Box<dyn Docker>,
    image: RunnableImage<I>,
    command: Command,
    ports: Ports,
}

impl<I> fmt::Debug for Container<I>
where
    I: fmt::Debug + Image,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Container")
            .field("id", &self.id)
            .field("image", &self.image)
            .field("command", &self.command)
            .finish()
    }
}

impl<I> Container<I>
where
    I: Image,
{
    /// Constructs a new container given an id, a docker client and the image.
    ///
    /// This function will block the current thread (if [`wait_until_ready`] is implemented correctly) until the container is actually ready to be used.
    ///
    /// [`wait_until_ready`]: trait.Image.html#tymethod.wait_until_ready
    pub(crate) fn new(
        id: String,
        docker_client: impl Docker + 'static,
        image: RunnableImage<I>,
        command: Command,
    ) -> Self {
        let ports = docker_client.ports(&id);
        Self {
            id,
            docker_client: Box::new(docker_client),
            image,
            command,
            ports,
        }
    }

    /// Returns a reference to the [`Image`] of this container.
    ///
    /// [`Image`]: trait.Image.html
    pub fn image(&self) -> &I {
        self.image.inner()
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

    pub fn ports(&self) -> Ports {
        self.ports.clone()
    }
}

impl<I> Container<I>
where
    I: Image,
{
    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        &self.id
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
    #[deprecated(
        since = "0.13.1",
        note = "Use `get_host_port_ipv4()` or `get_host_port_ipv6()` instead."
    )]
    pub fn get_host_port(&self, internal_port: u16) -> u16 {
        self.get_host_port_ipv4(internal_port)
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
    pub fn get_host_port_ipv4(&self, internal_port: u16) -> u16 {
        self.ports
            .map_to_host_port_ipv4(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "container {} does not expose port {}",
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
    pub fn get_host_port_ipv6(&self, internal_port: u16) -> u16 {
        self.ports
            .map_to_host_port_ipv6(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "container {} does not expose port {}",
                    self.id, internal_port
                )
            })
    }

    /// Returns the bridge ip address of docker container as specified in NetworkSettings.Networks.IPAddress
    pub fn get_bridge_ip_address(&self) -> IpAddr {
        let settings = self
            .docker_client
            .inspect(&self.id)
            .network_settings
            .unwrap_or_else(|| panic!("container {} has no network settings", self.id));

        let mut networks = settings
            .networks
            .unwrap_or_else(|| panic!("container {} has no any networks", self.id));

        let bridge_name = self
            .image
            .network()
            .clone()
            .or(settings.bridge)
            .unwrap_or_else(|| panic!("container {} has missing bridge name", self.id));

        let ip = networks
            .remove(&bridge_name)
            .and_then(|network| network.ip_address)
            .unwrap_or_else(|| panic!("container {} has missing bridge IP", self.id));

        IpAddr::from_str(&ip)
            .unwrap_or_else(|_| panic!("container {} has invalid bridge IP", self.id))
    }

    pub fn exec(&self, cmd: ExecCommand) -> ExecOutput {
        let ExecCommand {
            cmd,
            ready_conditions,
        } = cmd;

        log::debug!("Executing command {:?}", cmd);

        let output = self.docker_client.exec(self.id(), cmd);

        self.docker_client
            .block_until_ready(self.id(), ready_conditions);

        ExecOutput {
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }

    pub fn stop(&self) {
        log::debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id)
    }

    pub fn start(&self) {
        self.docker_client.start(&self.id);
    }

    pub fn rm(&self) {
        log::debug!("Deleting docker container {}", self.id);

        self.docker_client.rm(&self.id)
    }
}

/// Represents an output of `exec` command.
/// `stdout` & `stderr` is represented as bytes because it might contain non-utf chars and responsibility should be on the caller end.
#[derive(Debug)]
pub struct ExecOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// The destructor implementation for a Container.
///
/// As soon as the container goes out of scope, the destructor will either only stop or delete the docker container, depending on the [`Command`] value.
///
/// Setting it to `keep` will stop container.
/// Setting it to `remove` will remove it.
impl<I> Drop for Container<I>
where
    I: Image,
{
    fn drop(&mut self) {
        match self.command {
            Command::Keep => {}
            Command::Remove => self.rm(),
        }
        #[cfg(feature = "watchdog")]
        crate::watchdog::unregister(self.id());
    }
}

/// Defines operations that we need to perform on docker containers and other entities.
///
/// This trait is pub(crate) because it should not be used directly by users but only represents an internal abstraction that allows containers to be generic over the client they have been started with.
/// All functionality of this trait is available on [`Container`]s directly.
pub(crate) trait Docker: Sync + Send {
    fn stdout_logs(&self, id: &str) -> LogStream;
    fn stderr_logs(&self, id: &str) -> LogStream;
    fn ports(&self, id: &str) -> Ports;
    fn inspect(&self, id: &str) -> ContainerInspectResponse;
    fn rm(&self, id: &str);
    fn stop(&self, id: &str);
    fn start(&self, id: &str);
    fn exec(&self, id: &str, cmd: String) -> std::process::Output;
    fn block_until_ready(&self, id: &str, ready_conditions: Vec<WaitFor>);
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Default)]
    pub struct HelloWorld;

    impl Image for HelloWorld {
        type Args = ();

        fn name(&self) -> String {
            "hello-world".to_owned()
        }

        fn tag(&self) -> String {
            "latest".to_owned()
        }

        fn ready_conditions(&self) -> Vec<WaitFor> {
            vec![WaitFor::message_on_stdout("Hello from Docker!")]
        }
    }

    #[test]
    fn container_should_be_send_and_sync() {
        assert_send_and_sync::<Container<HelloWorld>>();
    }

    fn assert_send_and_sync<T: Send + Sync>() {}
}
