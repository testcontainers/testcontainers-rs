use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::{Debug, Formatter},
    net::IpAddr,
    time::Duration,
};

#[cfg(feature = "device-requests")]
use bollard::models::DeviceRequest;
use bollard::models::ResourcesUlimits;

use crate::{
    core::{
        copy::CopyToContainer, healthcheck::Healthcheck, logs::consumer::LogConsumer,
        mounts::Mount, ports::ContainerPort, ContainerState, ExecCommand, WaitFor,
    },
    Image, TestcontainersError,
};

/// Represents a request to start a container, allowing customization of the container.
#[must_use]
pub struct ContainerRequest<I: Image> {
    pub(crate) image: I,
    pub(crate) overridden_cmd: Vec<String>,
    pub(crate) image_name: Option<String>,
    pub(crate) image_tag: Option<String>,
    pub(crate) container_name: Option<String>,
    pub(crate) platform: Option<String>,
    pub(crate) network: Option<String>,
    pub(crate) hostname: Option<String>,
    pub(crate) labels: BTreeMap<String, String>,
    pub(crate) env_vars: BTreeMap<String, String>,
    pub(crate) hosts: BTreeMap<String, Host>,
    pub(crate) mounts: Vec<Mount>,
    pub(crate) copy_to_sources: Vec<CopyToContainer>,
    pub(crate) ports: Option<Vec<PortMapping>>,
    #[cfg(feature = "host-port-exposure")]
    pub(crate) host_port_exposures: Option<Vec<u16>>,
    pub(crate) ulimits: Option<Vec<ResourcesUlimits>>,
    pub(crate) privileged: bool,
    pub(crate) cap_add: Option<Vec<String>>,
    pub(crate) cap_drop: Option<Vec<String>>,
    pub(crate) readonly_rootfs: bool,
    pub(crate) security_opts: Option<Vec<String>>,
    pub(crate) shm_size: Option<u64>,
    pub(crate) cgroupns_mode: Option<CgroupnsMode>,
    pub(crate) userns_mode: Option<String>,
    pub(crate) startup_timeout: Option<Duration>,
    pub(crate) working_dir: Option<String>,
    pub(crate) log_consumers: Vec<Box<dyn LogConsumer + 'static>>,

    /// The length of a CPU period in microseconds. Default is 100000, this configures how
    /// CFS will schedule the threads for this container. Normally you don't adjust this and
    /// just set the CPU quota or nano CPUs. You might want to set this if you want to increase
    /// or reduce context-switching the container is subjected to.
    pub(crate) cpu_period: Option<i64>,

    /// Microseconds of CPU time that the container can get in a CPU period.
    /// Most users will want to set CPU quota to their desired CPU count * 100000.
    /// For example, to limit a container to 2 CPUs, set CPU quota to 200000.
    /// This is based on the default CPU period of 100000.
    /// If CPU quota is set to 0, the container will not be limited.
    pub(crate) cpu_quota: Option<i64>,

    /// The length of a CPU real-time period in microseconds. Set to 0 to allocate no time allocated to real-time tasks.
    pub(crate) cpu_realtime_period: Option<i64>,

    /// The length of a CPU real-time runtime in microseconds. Set to 0 to allocate no time allocated to real-time tasks.
    pub(crate) cpu_realtime_runtime: Option<i64>,

    /// CPUs in which to allow execution (e.g., `0-3`, `0,1`).
    /// Core pinning should help with performance consistency and context switching in some cases.
    pub(crate) cpuset_cpus: Option<String>,

    /// CPU quota in units of 10<sup>-9</sup> CPUs. This is basically what the --cpus flag turns into, but the
    /// raw value is denominated in billionths of a CPU. cpu_period and cpu_quota give you more control over the scheduler.
    pub nano_cpus: Option<i64>,

    /// Memory limit for the container, the _minimum_ is 6 MiB.
    /// This is the same as `HostConfig::memory`.
    pub(crate) memory: Option<i64>,

    /// Memory reservation, soft limit. Analogous to the JVM's `-Xms` option.
    /// The _minimum_ is 6 MiB.
    /// This is the same as `HostConfig::memory_reservation`.
    pub(crate) memory_reservation: Option<i64>,

    /// Total memory limit (memory + swap). Set as `-1` to enable unlimited swap.
    /// Same 6 MiB minimum as `memory`.
    pub memory_swap: Option<i64>,

    /// Tune a container's memory swappiness behavior. Accepts an integer between 0 and 100.
    pub memory_swappiness: Option<i64>,

    /// Disable OOM Killer for the container. This will not do anything unless -m (memory limit, cf. memory on this struct) is set.
    /// You can disable OOM-killer by writing "1" to memory.oom_control file, as:
    /// ```ignore
    /// echo 1 > memory.oom_control
    /// ```
    /// This operation is only allowed to the top cgroup of sub-hierarchy.
    /// If OOM-killer is disabled, tasks under cgroup will hang/sleep
    /// in memory cgroup's OOM-waitqueue when they request accountable memory.
    /// https://lwn.net/Articles/432224/
    pub oom_kill_disable: Option<bool>,

    /// Tune a container's PIDs limit. Set `0` or `-1` for unlimited, or `null` to not change.
    pub pids_limit: Option<i64>,

    #[cfg(feature = "reusable-containers")]
    pub(crate) reuse: crate::ReuseDirective,
    pub(crate) user: Option<String>,
    pub(crate) ready_conditions: Option<Vec<WaitFor>>,
    pub(crate) health_check: Option<Healthcheck>,
    #[cfg(feature = "device-requests")]
    pub(crate) device_requests: Option<Vec<DeviceRequest>>,
}

/// Represents a port mapping between a host's external port and the internal port of a container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortMapping {
    pub(crate) host_port: u16,
    pub(crate) container_port: ContainerPort,
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

    pub fn labels(&self) -> &BTreeMap<String, String> {
        &self.labels
    }

    pub fn container_name(&self) -> &Option<String> {
        &self.container_name
    }

    pub fn platform(&self) -> &Option<String> {
        &self.platform
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

    pub fn copy_to_sources(&self) -> impl Iterator<Item = &CopyToContainer> {
        self.image
            .copy_to_sources()
            .into_iter()
            .chain(self.copy_to_sources.iter())
    }

    pub fn ports(&self) -> Option<&Vec<PortMapping>> {
        self.ports.as_ref()
    }

    #[cfg(feature = "host-port-exposure")]
    pub fn host_port_exposures(&self) -> Option<&[u16]> {
        self.host_port_exposures.as_deref()
    }

    pub fn privileged(&self) -> bool {
        self.privileged
    }

    pub fn cap_add(&self) -> Option<&Vec<String>> {
        self.cap_add.as_ref()
    }

    pub fn cap_drop(&self) -> Option<&Vec<String>> {
        self.cap_drop.as_ref()
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
        self.ready_conditions
            .clone()
            .unwrap_or_else(|| self.image.ready_conditions())
    }

    pub fn expose_ports(&self) -> &[ContainerPort] {
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

    pub fn working_dir(&self) -> Option<&str> {
        self.working_dir.as_deref()
    }

    pub fn cpu_period(&self) -> Option<i64> {
        self.cpu_period
    }
    pub fn cpu_quota(&self) -> Option<i64> {
        self.cpu_quota
    }
    pub fn cpu_realtime_period(&self) -> Option<i64> {
        self.cpu_realtime_period
    }
    pub fn cpu_realtime_runtime(&self) -> Option<i64> {
        self.cpu_realtime_runtime
    }
    pub fn cpuset_cpus(&self) -> Option<&str> {
        self.cpuset_cpus.as_deref()
    }
    pub fn nano_cpus(&self) -> Option<i64> {
        self.nano_cpus
    }
    pub fn memory(&self) -> Option<i64> {
        self.memory
    }
    pub fn memory_reservation(&self) -> Option<i64> {
        self.memory_reservation
    }
    pub fn memory_swap(&self) -> Option<i64> {
        self.memory_swap
    }
    pub fn memory_swappiness(&self) -> Option<i64> {
        self.memory_swappiness
    }
    pub fn oom_kill_disable(&self) -> Option<bool> {
        self.oom_kill_disable
    }
    pub fn pids_limit(&self) -> Option<i64> {
        self.pids_limit
    }
    /// Indicates that the container will not be stopped when it is dropped
    #[cfg(feature = "reusable-containers")]
    pub fn reuse(&self) -> crate::ReuseDirective {
        self.reuse
    }

    /// Returns the configured user that commands are run as inside the container.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub fn security_opts(&self) -> Option<&Vec<String>> {
        self.security_opts.as_ref()
    }

    pub fn readonly_rootfs(&self) -> bool {
        self.readonly_rootfs
    }

    pub fn hostname(&self) -> Option<&str> {
        self.hostname.as_deref()
    }

    /// Returns the custom health check configuration for the container.
    pub fn health_check(&self) -> Option<&Healthcheck> {
        self.health_check.as_ref()
    }

    #[cfg(feature = "device-requests")]
    pub fn device_requests(&self) -> Option<&[DeviceRequest]> {
        self.device_requests.as_deref()
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
            platform: None,
            network: None,
            hostname: None,
            labels: BTreeMap::default(),
            env_vars: BTreeMap::default(),
            hosts: BTreeMap::default(),
            mounts: Vec::new(),
            copy_to_sources: Vec::new(),
            ports: None,
            #[cfg(feature = "host-port-exposure")]
            host_port_exposures: None,
            ulimits: None,
            privileged: false,
            cap_add: None,
            cap_drop: None,
            security_opts: None,
            readonly_rootfs: false,
            shm_size: None,
            cgroupns_mode: None,
            userns_mode: None,
            startup_timeout: None,
            working_dir: None,
            log_consumers: vec![],
            cpu_period: None,
            cpu_quota: None,
            cpu_realtime_period: None,
            cpu_realtime_runtime: None,
            cpuset_cpus: None,
            nano_cpus: None,
            memory: None,
            memory_reservation: None,
            memory_swap: None,
            memory_swappiness: None,
            oom_kill_disable: None,
            pids_limit: None,
            #[cfg(feature = "reusable-containers")]
            reuse: crate::ReuseDirective::Never,
            user: None,
            ready_conditions: None,
            health_check: None,
            #[cfg(feature = "device-requests")]
            device_requests: None,
        }
    }
}

impl PortMapping {
    pub(crate) fn new(local: u16, internal: ContainerPort) -> Self {
        Self {
            host_port: local,
            container_port: internal,
        }
    }

    pub fn host_port(&self) -> u16 {
        self.host_port
    }

    pub fn container_port(&self) -> ContainerPort {
        self.container_port
    }
}

impl<I: Image + Debug> Debug for ContainerRequest<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut repr = f.debug_struct("ContainerRequest");

        repr.field("image", &self.image)
            .field("overridden_cmd", &self.overridden_cmd)
            .field("image_name", &self.image_name)
            .field("image_tag", &self.image_tag)
            .field("container_name", &self.container_name)
            .field("platform", &self.platform)
            .field("network", &self.network)
            .field("hostname", &self.hostname)
            .field("labels", &self.labels)
            .field("env_vars", &self.env_vars)
            .field("hosts", &self.hosts)
            .field("mounts", &self.mounts)
            .field("ports", &self.ports);

        #[cfg(feature = "host-port-exposure")]
        repr.field("host_port_exposures", &self.host_port_exposures);

        repr.field("ulimits", &self.ulimits)
            .field("privileged", &self.privileged)
            .field("cap_add", &self.cap_add)
            .field("cap_drop", &self.cap_drop)
            .field("shm_size", &self.shm_size)
            .field("cgroupns_mode", &self.cgroupns_mode)
            .field("userns_mode", &self.userns_mode)
            .field("startup_timeout", &self.startup_timeout)
            .field("working_dir", &self.working_dir)
            .field("user", &self.user)
            .field("ready_conditions", &self.ready_conditions)
            .field("health_check", &self.health_check)
            .field("cpu_period", &self.cpu_period)
            .field("cpu_quota", &self.cpu_quota)
            .field("cpu_realtime_period", &self.cpu_realtime_period)
            .field("cpu_realtime_runtime", &self.cpu_realtime_runtime)
            .field("cpuset_cpus", &self.cpuset_cpus);
        #[cfg(feature = "reusable-containers")]
        repr.field("reusable", &self.reuse);

        #[cfg(feature = "device-requests")]
        repr.field("device_requests", &self.device_requests);

        repr.finish()
    }
}
