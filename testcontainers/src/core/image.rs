use std::{collections::BTreeMap, env::var, fmt::Debug, time::Duration};

use super::ports::Ports;

/// Represents a docker image.
///
/// Implementations are required to implement Default. The default instance of an [`Image`]
/// should have a meaningful configuration! It should be possible to [`run`][docker_run] the default
/// instance of an Image and get back a working container!
///
/// [`Image`]: trait.Image.html
/// [docker_run]: trait.Docker.html#tymethod.run
pub trait Image
where
    Self: Sized,
    Self::Args: ImageArgs + Clone + Debug,
{
    /// A type representing the arguments for an Image.
    ///
    /// There are a couple of things regarding the arguments of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its arguments should be meaningful!
    /// 2. Implementations should be conservative about which arguments they expose. Many times,
    /// users will either go with the default arguments or just override one or two. When defining
    /// the arguments of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    type Args;

    /// The name of the docker image to pull from the Docker Hub registry.
    fn name(&self) -> String;

    /// Implementations are encouraged to include a tag that will not change (i.e. NOT latest)
    /// in order to prevent test code from randomly breaking because the underlying docker
    /// suddenly changed.
    fn tag(&self) -> String;

    /// Returns a list of conditions that need to be met before a started container is considered ready.
    ///
    /// This method is the **🍞 and butter** of the whole testcontainers library. Containers are
    /// rarely instantly available as soon as they are started. Most of them take some time to boot
    /// up.
    ///
    /// The conditions returned from this method are evaluated **in the order** they are returned. Therefore
    /// you most likely want to start with a [`WaitFor::StdOutMessage`] or [`WaitFor::StdErrMessage`] and
    /// potentially follow up with a [`WaitFor::Duration`] in case the container usually needs a little
    /// more time before it is ready.
    fn ready_conditions(&self) -> Vec<WaitFor>;

    /// There are a couple of things regarding the environment variables of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its environment variables should be meaningful!
    /// 2. Implementations should be conservative about which environment variables they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the environment variables of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(std::iter::empty())
    }

    /// There are a couple of things regarding the volumes of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its volumes should be meaningful!
    /// 2. Implementations should be conservative about which volumes they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the volumes of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(std::iter::empty())
    }

    /// Returns the entrypoint this instance was created with.
    fn entrypoint(&self) -> Option<String> {
        None
    }

    /// Returns the ports that needs to be exposed when a container is created.
    ///
    /// This method is useful when there is a need to expose some ports, but there is
    /// no EXPOSE instruction in the Dockerfile of an image.
    fn expose_ports(&self) -> Vec<u16> {
        Default::default()
    }

    /// Returns the commands that needs to be executed after a container is started i.e. commands
    /// to be run in a running container.
    ///
    /// This method is useful when certain re-configuration is required after the start
    /// of container for the container to be considered ready for use in tests.
    #[allow(unused_variables)]
    fn exec_after_start(&self, cs: ContainerState) -> Vec<ExecCommand> {
        Default::default()
    }
}

#[derive(Default, Debug)]
pub struct ExecCommand {
    pub cmd: String,
    pub ready_conditions: Vec<WaitFor>,
}

#[derive(Debug)]
pub struct ContainerState {
    ports: Ports,
}

impl ContainerState {
    pub fn new(ports: Ports) -> Self {
        Self { ports }
    }

    #[deprecated(
        since = "0.13.1",
        note = "Use `host_port_ipv4()` or `host_port_ipv6()` instead."
    )]
    pub fn host_port(&self, internal_port: u16) -> u16 {
        self.host_port_ipv4(internal_port)
    }

    pub fn host_port_ipv4(&self, internal_port: u16) -> u16 {
        self.ports
            .map_to_host_port_ipv4(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "Container does not have a mapped port for {}",
                    internal_port
                )
            })
    }

    pub fn host_port_ipv6(&self, internal_port: u16) -> u16 {
        self.ports
            .map_to_host_port_ipv6(internal_port)
            .unwrap_or_else(|| {
                panic!(
                    "Container does not have a mapped port for {}",
                    internal_port
                )
            })
    }
}

pub trait ImageArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>>;
}

impl ImageArgs for () {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(vec![].into_iter())
    }
}

#[must_use]
#[derive(Debug)]
pub struct RunnableImage<I: Image> {
    image: I,
    image_args: I::Args,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    env_vars: BTreeMap<String, String>,
    volumes: BTreeMap<String, String>,
    ports: Option<Vec<Port>>,
}

impl<I: Image> RunnableImage<I> {
    pub fn inner(&self) -> &I {
        &self.image
    }

    pub fn args(&self) -> &I::Args {
        &self.image_args
    }

    pub fn network(&self) -> &Option<String> {
        &self.network
    }

    pub fn container_name(&self) -> &Option<String> {
        &self.container_name
    }

    pub fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.image.env_vars().chain(self.env_vars.iter()))
    }

    pub fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.image.volumes().chain(self.volumes.iter()))
    }

    pub fn ports(&self) -> &Option<Vec<Port>> {
        &self.ports
    }

    pub fn entrypoint(&self) -> Option<String> {
        self.image.entrypoint()
    }

    pub fn descriptor(&self) -> String {
        if let Some(tag) = &self.image_tag {
            format!("{}:{}", self.image.name(), tag)
        } else {
            format!("{}:{}", self.image.name(), self.image.tag())
        }
    }

    pub fn ready_conditions(&self) -> Vec<WaitFor> {
        self.image.ready_conditions()
    }

    pub fn expose_ports(&self) -> Vec<u16> {
        self.image.expose_ports()
    }

    pub fn exec_after_start(&self, cs: ContainerState) -> Vec<ExecCommand> {
        self.image.exec_after_start(cs)
    }
}

impl<I: Image> RunnableImage<I> {
    /// There is no guarantee that the specified tag for an image would result in a
    /// running container. Users of this API are advised to use this at their own risk.
    pub fn with_tag(self, tag: impl Into<String>) -> Self {
        Self {
            image_tag: Some(tag.into()),
            ..self
        }
    }

    pub fn with_container_name(self, name: impl Into<String>) -> Self {
        Self {
            container_name: Some(name.into()),
            ..self
        }
    }

    pub fn with_network(self, network: impl Into<String>) -> Self {
        Self {
            network: Some(network.into()),
            ..self
        }
    }

    pub fn with_env_var(self, (key, value): (impl Into<String>, impl Into<String>)) -> Self {
        let mut env_vars = self.env_vars;
        env_vars.insert(key.into(), value.into());
        Self { env_vars, ..self }
    }

    pub fn with_volume(self, (orig, dest): (impl Into<String>, impl Into<String>)) -> Self {
        let mut volumes = self.volumes;
        volumes.insert(orig.into(), dest.into());
        Self { volumes, ..self }
    }

    pub fn with_mapped_port<P: Into<Port>>(self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());

        Self {
            ports: Some(ports),
            ..self
        }
    }
}

impl<I> From<I> for RunnableImage<I>
where
    I: Image,
    I::Args: Default,
{
    fn from(image: I) -> Self {
        Self::from((image, I::Args::default()))
    }
}

impl<I: Image> From<(I, I::Args)> for RunnableImage<I> {
    fn from((image, image_args): (I, I::Args)) -> Self {
        Self {
            image,
            image_args,
            image_tag: None,
            container_name: None,
            network: None,
            env_vars: BTreeMap::default(),
            volumes: BTreeMap::default(),
            ports: None,
        }
    }
}

/// Represents a port mapping between a local port and the internal port of a container.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub local: u16,
    pub internal: u16,
}

/// Represents a condition that needs to be met before a container is considered ready.
#[derive(Debug, PartialEq, Clone)]
pub enum WaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a message on the stdout stream of the container's logs.
    StdOutMessage { message: String },
    /// Wait for a message on the stderr stream of the container's logs.
    StdErrMessage { message: String },
    /// Wait for a certain amount of time.
    Duration { length: Duration },
    /// Wait for the container's status to become `healthy`.
    Healthcheck,
}

impl WaitFor {
    pub fn message_on_stdout<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::StdOutMessage {
            message: message.into(),
        }
    }

    pub fn message_on_stderr<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::StdErrMessage {
            message: message.into(),
        }
    }

    pub fn seconds(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_secs(length),
        }
    }

    pub fn millis(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_millis(length),
        }
    }

    pub fn millis_in_env_var(name: &'static str) -> WaitFor {
        let additional_sleep_period = var(name).map(|value| value.parse());

        (|| {
            let length = additional_sleep_period.ok()?.ok()?;

            Some(WaitFor::Duration {
                length: Duration::from_millis(length),
            })
        })()
        .unwrap_or(WaitFor::Nothing)
    }
}

impl From<(u16, u16)> for Port {
    fn from((local, internal): (u16, u16)) -> Self {
        Port { local, internal }
    }
}
