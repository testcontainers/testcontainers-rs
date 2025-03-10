use std::time::Duration;

use bollard_stubs::models::ResourcesUlimits;

use crate::{
    core::{
        copy::{CopyDataSource, CopyToContainer},
        logs::consumer::LogConsumer,
        CgroupnsMode, ContainerPort, Host, Mount, PortMapping,
    },
    ContainerRequest, Image,
};

#[cfg(feature = "reusable-containers")]
#[derive(Eq, Copy, Clone, Debug, Default, PartialEq)]
pub enum ReuseDirective {
    #[default]
    Never,
    Always,
    CurrentSession,
}

#[cfg(feature = "reusable-containers")]
impl std::fmt::Display for ReuseDirective {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Never => "never",
            Self::Always => "always",
            Self::CurrentSession => "current-session",
        })
    }
}

/// Represents an extension for the [`Image`] trait.
/// Allows to override image defaults and container configuration.
pub trait ImageExt<I: Image> {
    /// Returns a new [`ContainerRequest`] with the specified (overridden) `CMD` ([`Image::cmd`]).
    ///
    /// # Examples
    /// ```rust,no_run
    /// use testcontainers::{GenericImage, ImageExt};
    ///
    /// let image = GenericImage::new("image", "tag");
    /// let cmd = ["arg1", "arg2"];
    /// let overridden_cmd = image.clone().with_cmd(cmd);
    ///
    /// assert!(overridden_cmd.cmd().eq(cmd));
    ///
    /// let another_container_req = image.with_cmd(cmd);
    ///
    /// assert!(another_container_req.cmd().eq(overridden_cmd.cmd()));
    /// ```
    fn with_cmd(self, cmd: impl IntoIterator<Item = impl Into<String>>) -> ContainerRequest<I>;

    /// Overrides the fully qualified image name (consists of `{domain}/{owner}/{image}`).
    /// Can be used to specify a custom registry or owner.
    fn with_name(self, name: impl Into<String>) -> ContainerRequest<I>;

    /// Overrides the image tag.
    ///
    /// There is no guarantee that the specified tag for an image would result in a
    /// running container. Users of this API are advised to use this at their own risk.
    fn with_tag(self, tag: impl Into<String>) -> ContainerRequest<I>;

    /// Sets the container name.
    fn with_container_name(self, name: impl Into<String>) -> ContainerRequest<I>;

    /// Sets the network the container will be connected to.
    fn with_network(self, network: impl Into<String>) -> ContainerRequest<I>;

    /// Adds the specified label to the container.
    ///
    /// **Note**: all keys in the `org.testcontainers.*` namespace should be regarded
    /// as reserved by `testcontainers` internally, and should not be expected or relied
    /// upon to be applied correctly if supplied as a value for `key`.
    fn with_label(self, key: impl Into<String>, value: impl Into<String>) -> ContainerRequest<I>;

    /// Adds the specified labels to the container.
    ///
    /// **Note**: all keys in the `org.testcontainers.*` namespace should be regarded
    /// as reserved by `testcontainers` internally, and should not be expected or relied
    /// upon to be applied correctly if they are included in `labels`.
    fn with_labels(
        self,
        labels: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> ContainerRequest<I>;

    /// Adds an environment variable to the container.
    fn with_env_var(self, name: impl Into<String>, value: impl Into<String>)
        -> ContainerRequest<I>;

    /// Adds a host to the container.
    fn with_host(self, key: impl Into<String>, value: impl Into<Host>) -> ContainerRequest<I>;

    /// Adds a mount to the container.
    fn with_mount(self, mount: impl Into<Mount>) -> ContainerRequest<I>;

    /// Copies some source into the container as file
    fn with_copy_to(
        self,
        target: impl Into<String>,
        source: impl Into<CopyDataSource>,
    ) -> ContainerRequest<I>;

    /// Adds a port mapping to the container, mapping the host port to the container's internal port.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use testcontainers::{GenericImage, ImageExt};
    /// use testcontainers::core::IntoContainerPort;
    ///
    /// let image = GenericImage::new("image", "tag").with_mapped_port(8080, 80.tcp());
    /// ```
    fn with_mapped_port(self, host_port: u16, container_port: ContainerPort)
        -> ContainerRequest<I>;

    /// Adds a resource ulimit to the container.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use testcontainers::{GenericImage, ImageExt};
    ///
    /// let image = GenericImage::new("image", "tag").with_ulimit("nofile", 65536, Some(65536));
    /// ```
    fn with_ulimit(self, name: &str, soft: i64, hard: Option<i64>) -> ContainerRequest<I>;

    /// Sets the container to run in privileged mode.
    fn with_privileged(self, privileged: bool) -> ContainerRequest<I>;

    /// Adds the capabilities to the container
    fn with_cap_add(self, capability: impl Into<String>) -> ContainerRequest<I>;

    /// Drops the capabilities from the container's capabilities
    fn with_cap_drop(self, capability: impl Into<String>) -> ContainerRequest<I>;

    /// cgroup namespace mode for the container. Possible values are:
    /// - [`CgroupnsMode::Private`]: the container runs in its own private cgroup namespace
    /// - [`CgroupnsMode::Host`]: use the host system's cgroup namespace
    ///
    /// If not specified, the daemon default is used, which can either be `\"private\"` or `\"host\"`, depending on daemon version, kernel support and configuration.
    fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> ContainerRequest<I>;

    /// Sets the usernamespace mode for the container when usernamespace remapping option is enabled.
    fn with_userns_mode(self, userns_mode: &str) -> ContainerRequest<I>;

    /// Sets the shared memory size in bytes
    fn with_shm_size(self, bytes: u64) -> ContainerRequest<I>;

    /// Sets the startup timeout for the container. The default is 60 seconds.
    fn with_startup_timeout(self, timeout: Duration) -> ContainerRequest<I>;

    /// Sets the working directory. The default is defined by the underlying image, which in turn may default to `/`.
    fn with_working_dir(self, working_dir: impl Into<String>) -> ContainerRequest<I>;

    /// Adds the log consumer to the container.
    ///
    /// Allows to follow the container logs for the whole lifecycle of the container, starting from the creation.
    fn with_log_consumer(self, log_consumer: impl LogConsumer + 'static) -> ContainerRequest<I>;

    /// Sets the CPU period for the container.
    /// The default is defined by the underlying image.
    /// The length of a CPU period in microseconds.
    /// https://docs.docker.com/engine/reference/commandline/run/#cpu-period
    fn with_cpu_period(self, cpu_period: impl Into<i64>) -> ContainerRequest<I>;

    /// Sets the CPU quota for the container.
    /// The default is defined by the underlying image.
    /// Microseconds of CPU time that the container can get in a CPU period.
    /// https://docs.docker.com/engine/reference/commandline/run/#cpu-quota
    /// Most users will want to set CPU quota to their desired CPU count * 100000.
    /// For example, to limit a container to 2 CPUs, set CPU quota to 200000.
    /// This is based on the default CPU period of 100000.
    /// If CPU quota is set to 0, the container will not be limited.
    fn with_cpu_quota(self, cpu_quota: impl Into<i64>) -> ContainerRequest<I>;

    /// Sets the CPU realtime period for the container.
    /// The default is defined by the underlying image.
    /// The length of a CPU real-time period in microseconds.
    fn with_cpu_realtime_period(self, cpu_realtime_period: impl Into<i64>) -> ContainerRequest<I>;

    /// Sets the CPU realtime runtime for the container.
    fn with_cpu_realtime_runtime(self, cpu_realtime_runtime: impl Into<i64>)
        -> ContainerRequest<I>;

    /// Sets the CPUs in which to allow execution (e.g., `0-3`, `0,1`).
    /// Core pinning should help with performance consistency and context switching in some cases.
    /// The default is defined by the underlying image.
    fn with_cpuset_cpus(self, cpuset_cpus: impl Into<String>) -> ContainerRequest<I>;

    /// Memory limit for the container, the _minimum_ is 6 MiB.
    /// This is the same as `HostConfig::memory`.
    fn with_memory(self, bytes: i64) -> ContainerRequest<I>;

    /// Memory reservation, soft limit. Analogous to the JVM's `-Xms` option.
    /// The _minimum_ is 6 MiB.
    /// This is the same as `HostConfig::memory_reservation`.
    fn with_memory_reservation(self, bytes: i64) -> ContainerRequest<I>;

    /// Total memory limit (memory + swap). Set as `-1` to enable unlimited swap.
    /// Same 6 MiB minimum as `memory`. I do not know why everything is i64.
    fn with_memory_swap(self, bytes: i64) -> ContainerRequest<I>;

    /// Tune a container's memory swappiness behavior. Accepts an integer between 0 and 100.
    fn with_memory_swappiness(self, swappiness: i64) -> ContainerRequest<I>;

    /// Disable OOM Killer for the container. This will not do anything unless -m (memory limit, cf. memory on this struct) is set.
    /// You can disable OOM-killer by writing "1" to memory.oom_control file, as:
    /// ```ignore
    /// echo 1 > memory.oom_control
    /// ```
    /// This operation is only allowed to the top cgroup of sub-hierarchy.
    /// If OOM-killer is disabled, tasks under cgroup will hang/sleep
    /// in memory cgroup's OOM-waitqueue when they request accountable memory.
    /// https://lwn.net/Articles/432224/
    fn with_oom_kill_disable(self, disable: bool) -> ContainerRequest<I>;

    /// Tune a container's PIDs limit. Set `0` or `-1` for unlimited, or `null` to not change.
    fn with_pids_limit(self, limit: i64) -> ContainerRequest<I>;

    /// Flag the container as being exempt from the default `testcontainers` remove-on-drop lifecycle,
    /// indicating that the container should be kept running, and that executions with the same configuration
    /// reuse it instead of starting a "fresh" container instance.
    ///
    /// **NOTE:** Reusable Containers is an experimental feature, and its behavior is therefore subject
    /// to change. Containers marked as `reuse` **_will not_** be stopped or cleaned up when their associated
    /// `Container` or `ContainerAsync` is dropped.
    #[cfg(feature = "reusable-containers")]
    fn with_reuse(self, reuse: ReuseDirective) -> ContainerRequest<I>;
}

/// Implements the [`ImageExt`] trait for the every type that can be converted into a [`ContainerRequest`].
impl<RI: Into<ContainerRequest<I>>, I: Image> ImageExt<I> for RI {
    fn with_cmd(self, cmd: impl IntoIterator<Item = impl Into<String>>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            overridden_cmd: cmd.into_iter().map(Into::into).collect(),
            ..container_req
        }
    }

    fn with_name(self, name: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            image_name: Some(name.into()),
            ..container_req
        }
    }

    fn with_tag(self, tag: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            image_tag: Some(tag.into()),
            ..container_req
        }
    }

    fn with_container_name(self, name: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();

        ContainerRequest {
            container_name: Some(name.into()),
            ..container_req
        }
    }

    fn with_network(self, network: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            network: Some(network.into()),
            ..container_req
        }
    }

    fn with_label(self, key: impl Into<String>, value: impl Into<String>) -> ContainerRequest<I> {
        let mut container_req = self.into();

        container_req.labels.insert(key.into(), value.into());

        container_req
    }

    fn with_labels(
        self,
        labels: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> ContainerRequest<I> {
        let mut container_req = self.into();

        container_req.labels.extend(
            labels
                .into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );

        container_req
    }

    fn with_env_var(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.env_vars.insert(name.into(), value.into());
        container_req
    }

    fn with_host(self, key: impl Into<String>, value: impl Into<Host>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.hosts.insert(key.into(), value.into());
        container_req
    }

    fn with_mount(self, mount: impl Into<Mount>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.mounts.push(mount.into());
        container_req
    }

    fn with_copy_to(
        self,
        target: impl Into<String>,
        source: impl Into<CopyDataSource>,
    ) -> ContainerRequest<I> {
        let mut container_req = self.into();
        let target: String = target.into();
        container_req
            .copy_to_sources
            .push(CopyToContainer::new(source, target));
        container_req
    }

    fn with_mapped_port(
        self,
        host_port: u16,
        container_port: ContainerPort,
    ) -> ContainerRequest<I> {
        let container_req = self.into();
        let mut ports = container_req.ports.unwrap_or_default();
        ports.push(PortMapping::new(host_port, container_port));

        ContainerRequest {
            ports: Some(ports),
            ..container_req
        }
    }

    fn with_ulimit(self, name: &str, soft: i64, hard: Option<i64>) -> ContainerRequest<I> {
        let container_req = self.into();
        let mut ulimits = container_req.ulimits.unwrap_or_default();
        ulimits.push(ResourcesUlimits {
            name: Some(name.into()),
            soft: Some(soft),
            hard,
        });

        ContainerRequest {
            ulimits: Some(ulimits),
            ..container_req
        }
    }

    fn with_privileged(self, privileged: bool) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            privileged,
            ..container_req
        }
    }

    fn with_cap_add(self, capability: impl Into<String>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req
            .cap_add
            .get_or_insert_with(Vec::new)
            .push(capability.into());

        container_req
    }

    fn with_cap_drop(self, capability: impl Into<String>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req
            .cap_drop
            .get_or_insert_with(Vec::new)
            .push(capability.into());

        container_req
    }

    fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            cgroupns_mode: Some(cgroupns_mode),
            ..container_req
        }
    }

    fn with_userns_mode(self, userns_mode: &str) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            userns_mode: Some(String::from(userns_mode)),
            ..container_req
        }
    }

    fn with_shm_size(self, bytes: u64) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            shm_size: Some(bytes),
            ..container_req
        }
    }

    fn with_startup_timeout(self, timeout: Duration) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            startup_timeout: Some(timeout),
            ..container_req
        }
    }

    fn with_working_dir(self, working_dir: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            working_dir: Some(working_dir.into()),
            ..container_req
        }
    }

    fn with_log_consumer(self, log_consumer: impl LogConsumer + 'static) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.log_consumers.push(Box::new(log_consumer));
        container_req
    }

    fn with_cpu_period(self, cpu_period: impl Into<i64>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            cpu_period: Some(cpu_period.into()),
            ..container_req
        }
    }

    fn with_cpu_quota(self, cpu_quota: impl Into<i64>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            cpu_quota: Some(cpu_quota.into()),
            ..container_req
        }
    }

    fn with_cpu_realtime_period(self, cpu_realtime_period: impl Into<i64>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            cpu_realtime_period: Some(cpu_realtime_period.into()),
            ..container_req
        }
    }

    fn with_cpu_realtime_runtime(
        self,
        cpu_realtime_runtime: impl Into<i64>,
    ) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            cpu_realtime_runtime: Some(cpu_realtime_runtime.into()),
            ..container_req
        }
    }

    fn with_cpuset_cpus(self, cpuset_cpus: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            cpuset_cpus: Some(cpuset_cpus.into()),
            ..container_req
        }
    }

    fn with_memory(self, bytes: i64) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            memory: Some(bytes),
            ..container_req
        }
    }

    fn with_memory_reservation(self, bytes: i64) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            memory_reservation: Some(bytes),
            ..container_req
        }
    }

    fn with_memory_swap(self, bytes: i64) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            memory_swap: Some(bytes),
            ..container_req
        }
    }

    fn with_memory_swappiness(self, swappiness: i64) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            memory_swappiness: Some(swappiness),
            ..container_req
        }
    }

    fn with_oom_kill_disable(self, disable: bool) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            oom_kill_disable: Some(disable),
            ..container_req
        }
    }

    fn with_pids_limit(self, limit: i64) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            pids_limit: Some(limit),
            ..container_req
        }
    }

    #[cfg(feature = "reusable-containers")]
    fn with_reuse(self, reuse: ReuseDirective) -> ContainerRequest<I> {
        ContainerRequest {
            reuse,
            ..self.into()
        }
    }
}
