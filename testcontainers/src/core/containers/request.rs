use std::{borrow::Cow, collections::BTreeMap, net::IpAddr, time::Duration};

use crate::{
    core::{mounts::Mount, ports::ExposedPort, ContainerState, ExecCommand, WaitFor},
    Image, TestcontainersError,
};

/// Represents a request to start a container, allowing customization of the container.
#[must_use]
#[derive(Debug, Clone)]
pub struct ContainerRequest<I: Image> {
    pub(crate) image: I,
    pub(crate) overridden_cmd: Vec<String>,
    pub(crate) image_name: Option<String>,
    pub(crate) image_tag: Option<String>,
    pub(crate) container_name: Option<String>,
    pub(crate) network: Option<String>,
    pub(crate) env_vars: BTreeMap<String, String>,
    pub(crate) hosts: BTreeMap<String, Host>,
    pub(crate) mounts: Vec<Mount>,
    pub(crate) ports: Option<Vec<PortMapping>>,
    pub(crate) privileged: bool,
    pub(crate) shm_size: Option<u64>,
    pub(crate) cgroupns_mode: Option<CgroupnsMode>,
    pub(crate) userns_mode: Option<String>,
    pub(crate) startup_timeout: Option<Duration>,
}

/// Represents a port mapping between a local port and the internal port of a container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortMapping {
    pub local: u16,
    pub internal: ExposedPort,
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
    /// Use the host system's cgroup namespace
    Host,
    /// Private cgroup namespace
    Private,
}

impl<I: Image> ContainerRequest<I> {
    pub fn image(&self) -> &I {
        &self.image
    }

    pub fn network(&self) -> &Option<String> {
        &self.network
    }

    pub fn container_name(&self) -> &Option<String> {
        &self.container_name
    }

    pub fn env_vars(&self) -> impl Iterator<Item = (Cow<'_, str>, Cow<'_, str>)> {
        self.image
            .env_vars()
            .into_iter()
            .map(|(name, val)| (name.into(), val.into()))
            .chain(
                self.env_vars
                    .iter()
                    .map(|(name, val)| (name.into(), val.into())),
            )
    }

    pub fn hosts(&self) -> impl Iterator<Item = (Cow<'_, str>, &Host)> {
        self.hosts.iter().map(|(name, host)| (name.into(), host))
    }

    pub fn mounts(&self) -> impl Iterator<Item = &Mount> {
        self.image.mounts().into_iter().chain(self.mounts.iter())
    }

    pub fn ports(&self) -> Option<&Vec<PortMapping>> {
        self.ports.as_ref()
    }

    pub fn privileged(&self) -> bool {
        self.privileged
    }

    pub fn cgroupns_mode(&self) -> Option<CgroupnsMode> {
        self.cgroupns_mode
    }

    pub fn userns_mode(&self) -> Option<&str> {
        self.userns_mode.as_deref()
    }

    /// Shared memory size in bytes
    pub fn shm_size(&self) -> Option<u64> {
        self.shm_size
    }

    pub fn entrypoint(&self) -> Option<&str> {
        self.image.entrypoint()
    }

    pub fn cmd(&self) -> impl Iterator<Item = Cow<'_, str>> {
        if !self.overridden_cmd.is_empty() {
            either::Either::Left(self.overridden_cmd.iter().map(Cow::from))
        } else {
            either::Either::Right(self.image.cmd().into_iter().map(Into::into))
        }
    }

    pub fn descriptor(&self) -> String {
        let original_name = self.image.name();
        let original_tag = self.image.tag();

        let name = self.image_name.as_deref().unwrap_or(original_name);
        let tag = self.image_tag.as_deref().unwrap_or(original_tag);

        format!("{name}:{tag}")
    }

    pub fn ready_conditions(&self) -> Vec<WaitFor> {
        self.image.ready_conditions()
    }

    pub fn expose_ports(&self) -> &[ExposedPort] {
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

impl<I: Image> From<I> for ContainerRequest<I> {
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

impl From<(u16, ExposedPort)> for PortMapping {
    fn from((local, internal): (u16, ExposedPort)) -> Self {
        PortMapping { local, internal }
    }
}
