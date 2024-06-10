use std::{collections::BTreeMap, net::IpAddr, time::Duration};

use crate::{
    core::{mounts::Mount, ContainerState, ExecCommand, WaitFor},
    Image, TestcontainersError,
};

/// Image wrapper that allows to override some of the image properties.
#[must_use]
#[derive(Debug, Clone)]
pub struct RunnableImage<I: Image> {
    image: I,
    overridden_cmd: Vec<String>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    env_vars: BTreeMap<String, String>,
    hosts: BTreeMap<String, Host>,
    mounts: Vec<Mount>,
    ports: Option<Vec<PortMapping>>,
    privileged: bool,
    shm_size: Option<u64>,
    cgroupns_mode: Option<CgroupnsMode>,
    userns_mode: Option<String>,
    startup_timeout: Option<Duration>,
}

/// Represents a port mapping between a local port and the internal port of a container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortMapping {
    pub local: u16,
    pub internal: u16,
}

#[derive(parse_display::Display, Debug, Clone)]
pub enum Host {
    #[display("{0}")]
    Addr(IpAddr),
    #[display("host-gateway")]
    HostGateway,
}

#[derive(Debug, Clone, Copy)]
pub enum CgroupnsMode {
    Host,
    Private,
}

impl<I: Image> RunnableImage<I> {
    pub fn image(&self) -> &I {
        &self.image
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

    pub fn hosts(&self) -> Box<dyn Iterator<Item = (&String, &Host)> + '_> {
        Box::new(self.hosts.iter())
    }

    pub fn mounts(&self) -> Box<dyn Iterator<Item = &Mount> + '_> {
        Box::new(self.image.mounts().chain(self.mounts.iter()))
    }

    pub fn ports(&self) -> &Option<Vec<PortMapping>> {
        &self.ports
    }

    pub fn privileged(&self) -> bool {
        self.privileged
    }

    pub fn cgroupns_mode(&self) -> Option<CgroupnsMode> {
        self.cgroupns_mode
    }

    pub fn userns_mode(&self) -> Option<String> {
        self.userns_mode.clone()
    }

    /// Shared memory size in bytes
    pub fn shm_size(&self) -> Option<u64> {
        self.shm_size
    }

    pub fn entrypoint(&self) -> Option<String> {
        self.image.entrypoint()
    }

    pub fn cmd(&self) -> Vec<String> {
        if !self.overridden_cmd.is_empty() {
            self.overridden_cmd.clone()
        } else {
            self.image.cmd().into_iter().map(Into::into).collect()
        }
    }

    pub fn descriptor(&self) -> String {
        let original_name = self.image.name();
        let original_tag = self.image.tag();

        let name = self.image_name.as_ref().unwrap_or(&original_name);
        let tag = self.image_tag.as_ref().unwrap_or(&original_tag);

        format!("{name}:{tag}")
    }

    pub fn ready_conditions(&self) -> Vec<WaitFor> {
        self.image.ready_conditions()
    }

    pub fn expose_ports(&self) -> Vec<u16> {
        self.image.expose_ports()
    }

    pub fn exec_after_start(
        &self,
        cs: ContainerState,
    ) -> Result<Vec<ExecCommand>, TestcontainersError> {
        self.image.exec_after_start(cs)
    }

    /// Returns the startup timeout for the container.
    pub fn startup_timeout(&self) -> Option<Duration> {
        self.startup_timeout
    }
}

impl<I: Image> RunnableImage<I> {
    /// Returns a new RunnableImage with the specified (overridden) `CMD` ([`Image::cmd`]).
    ///
    /// # Examples
    /// ```rust,no_run
    /// use testcontainers::{core::RunnableImage, GenericImage};
    ///
    /// let image = GenericImage::default();
    /// let cmd = ["arg1", "arg2"];
    /// let runnable_image = RunnableImage::from(image.clone()).with_cmd(cmd);
    ///
    /// assert_eq!(runnable_image.cmd(), &cmd);
    ///
    /// let another_runnable_image = RunnableImage::from((image, cmd));
    ///
    /// assert_eq!(another_runnable_image.cmd(), runnable_image.cmd());
    /// ```
    pub fn with_cmd(self, cmd: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            overridden_cmd: cmd.into_iter().map(Into::into).collect(),
            ..self
        }
    }

    /// Overrides the fully qualified image name (consists of `{domain}/{owner}/{image}`).
    /// Can be used to specify a custom registry or owner.
    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self {
            image_name: Some(name.into()),
            ..self
        }
    }

    /// Overrides the image tag.
    ///
    /// There is no guarantee that the specified tag for an image would result in a
    /// running container. Users of this API are advised to use this at their own risk.
    pub fn with_tag(self, tag: impl Into<String>) -> Self {
        Self {
            image_tag: Some(tag.into()),
            ..self
        }
    }

    /// Sets the container name.
    pub fn with_container_name(self, name: impl Into<String>) -> Self {
        Self {
            container_name: Some(name.into()),
            ..self
        }
    }

    /// Sets the network the container will be connected to.
    pub fn with_network(self, network: impl Into<String>) -> Self {
        Self {
            network: Some(network.into()),
            ..self
        }
    }

    /// Adds an environment variable to the container.
    pub fn with_env_var(mut self, (key, value): (impl Into<String>, impl Into<String>)) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Adds a host to the container.
    pub fn with_host(mut self, key: impl Into<String>, value: impl Into<Host>) -> Self {
        self.hosts.insert(key.into(), value.into());
        self
    }

    /// Adds a mount to the container.
    pub fn with_mount(mut self, mount: impl Into<Mount>) -> Self {
        self.mounts.push(mount.into());
        self
    }

    /// Adds a port mapping to the container.
    pub fn with_mapped_port<P: Into<PortMapping>>(self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());

        Self {
            ports: Some(ports),
            ..self
        }
    }

    /// Sets the container to run in privileged mode.
    pub fn with_privileged(self, privileged: bool) -> Self {
        Self { privileged, ..self }
    }

    /// cgroup namespace mode for the container. Possible values are:
    /// - `\"private\"`: the container runs in its own private cgroup namespace
    /// - `\"host\"`: use the host system's cgroup namespace
    /// If not specified, the daemon default is used, which can either be `\"private\"` or `\"host\"`, depending on daemon version, kernel support and configuration.
    pub fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> Self {
        Self {
            cgroupns_mode: Some(cgroupns_mode),
            ..self
        }
    }

    /// Sets the usernamespace mode for the container when usernamespace remapping option is enabled.
    pub fn with_userns_mode(self, userns_mode: &str) -> Self {
        Self {
            userns_mode: Some(String::from(userns_mode)),
            ..self
        }
    }

    /// Sets the shared memory size in bytes
    pub fn with_shm_size(self, bytes: u64) -> Self {
        Self {
            shm_size: Some(bytes),
            ..self
        }
    }

    /// Sets the startup timeout for the container. The default is 60 seconds.
    pub fn with_startup_timeout(self, timeout: Duration) -> Self {
        Self {
            startup_timeout: Some(timeout),
            ..self
        }
    }
}

impl<I: Image> From<I> for RunnableImage<I> {
    fn from(image: I) -> Self {
        Self {
            image,
            overridden_cmd: Vec::new(),
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            env_vars: BTreeMap::default(),
            hosts: BTreeMap::default(),
            mounts: Vec::new(),
            ports: None,
            privileged: false,
            shm_size: None,
            cgroupns_mode: None,
            userns_mode: None,
            startup_timeout: None,
        }
    }
}

impl From<(u16, u16)> for PortMapping {
    fn from((local, internal): (u16, u16)) -> Self {
        PortMapping { local, internal }
    }
}
