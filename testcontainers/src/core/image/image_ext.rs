use std::{sync::Arc, time::Duration};

#[cfg(feature = "device-requests")]
use bollard::models::DeviceRequest;
use bollard::models::{HostConfig, ResourcesUlimits};

use crate::{
    core::{
        copy::{CopyDataSource, CopyTargetOptions, CopyToContainer},
        healthcheck::Healthcheck,
        logs::consumer::LogConsumer,
        CgroupnsMode, ContainerPort, Host, Mount, PortMapping, WaitFor,
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

    /// Sets the platform the container will be run on.
    ///
    /// Platform in the format `os[/arch[/variant]]` used for image lookup.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use testcontainers::{GenericImage, ImageExt};
    ///
    /// let image = GenericImage::new("image", "tag")
    ///     .with_platform("linux/amd64");
    /// ```
    fn with_platform(self, platform: impl Into<String>) -> ContainerRequest<I>;

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

    /// Configures hostname for the container.
    fn with_hostname(self, hostname: impl Into<String>) -> ContainerRequest<I>;

    /// Adds a mount to the container.
    fn with_mount(self, mount: impl Into<Mount>) -> ContainerRequest<I>;

    /// Copies data or a file/dir into the container.
    ///
    /// The simplest form mirrors existing behavior:
    /// ```rust,no_run
    /// use std::path::Path;
    /// use testcontainers::{GenericImage, ImageExt};
    ///
    /// let image = GenericImage::new("image", "tag");
    /// image.with_copy_to("/app/config.toml", Path::new("./config.toml"));
    /// ```
    ///
    /// By default the target mode is derived from the source file's mode on Unix,
    /// and falls back to `0o644` on non-Unix platforms.
    ///
    /// To override the mode (or add more target options), wrap the target with
    /// [`CopyTargetOptions`]:
    /// ```rust,no_run
    /// use std::path::Path;
    /// use testcontainers::{CopyTargetOptions, GenericImage, ImageExt};
    ///
    /// let image = GenericImage::new("image", "tag");
    /// image.with_copy_to(
    ///     CopyTargetOptions::new("/app/config.toml").with_mode(0o600),
    ///     Path::new("./config.toml"),
    /// );
    /// ```
    fn with_copy_to(
        self,
        target: impl Into<CopyTargetOptions>,
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

    /// Declares a host port that should be reachable from inside the container.
    #[cfg(feature = "host-port-exposure")]
    fn with_exposed_host_port(self, port: u16) -> ContainerRequest<I>;

    /// Declares multiple host ports that should be reachable from inside the container.
    #[cfg(feature = "host-port-exposure")]
    fn with_exposed_host_ports(self, ports: impl IntoIterator<Item = u16>) -> ContainerRequest<I>;

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

    /// Applies a custom modifier to the Docker `HostConfig` used for container creation.
    ///
    /// The modifier runs after `testcontainers` finishes applying its defaults and settings.
    /// If called multiple times, the last modifier replaces the previous one.
    fn with_host_config_modifier(
        self,
        modifier: impl Fn(&mut HostConfig) + Send + Sync + 'static,
    ) -> ContainerRequest<I>;

    /// Flag the container as being exempt from the default `testcontainers` remove-on-drop lifecycle,
    /// indicating that the container should be kept running, and that executions with the same configuration
    /// reuse it instead of starting a "fresh" container instance.
    ///
    /// **NOTE:** Reusable Containers is an experimental feature, and its behavior is therefore subject
    /// to change. Containers marked as `reuse` **_will not_** be stopped or cleaned up when their associated
    /// `Container` or `ContainerAsync` is dropped.
    #[cfg(feature = "reusable-containers")]
    fn with_reuse(self, reuse: ReuseDirective) -> ContainerRequest<I>;

    /// Sets the user that commands are run as inside the container.
    fn with_user(self, user: impl Into<String>) -> ContainerRequest<I>;

    /// Sets the container's root filesystem to be mounted as read-only
    fn with_readonly_rootfs(self, readonly_rootfs: bool) -> ContainerRequest<I>;

    /// Sets security options for the container
    fn with_security_opt(self, security_opt: impl Into<String>) -> ContainerRequest<I>;

    /// Overrides ready conditions.
    ///
    /// There is no guarantee that the specified ready conditions for an image would result
    /// in a running container. Users of this API are advised to use this at their own risk.
    fn with_ready_conditions(self, ready_conditions: Vec<WaitFor>) -> ContainerRequest<I>;

    /// Sets a custom health check for the container.
    ///
    /// This will override any `HEALTHCHECK` instruction defined in the image.
    /// See [`Healthcheck`] for more details on how to build a health check.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use testcontainers::{core::{Healthcheck, WaitFor}, GenericImage, ImageExt};
    /// use std::time::Duration;
    ///
    /// let image = GenericImage::new("mysql", "8.0")
    ///     .with_wait_for(WaitFor::healthcheck())
    ///     .with_health_check(
    ///         Healthcheck::cmd(["mysqladmin", "ping", "-h", "localhost", "-u", "root", "-proot"])
    ///             .with_interval(Duration::from_secs(2))
    ///             .with_timeout(Duration::from_secs(1))
    ///             .with_retries(5)
    ///     );
    /// ```
    fn with_health_check(self, health_check: Healthcheck) -> ContainerRequest<I>;

    /// Injects device requests into the container request.
    ///
    /// This allows, for instance, exposing the underlying host's GPU:
    /// https://docs.docker.com/compose/how-tos/gpu-support/#example-of-a-compose-file-for-running-a-service-with-access-to-1-gpu-device
    ///
    /// This brings in 2 requirements:
    ///
    /// - The host must have [NVIDIA container toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html) installed.
    /// - The image must have [NVIDIA drivers](https://www.nvidia.com/en-us/drivers/) installed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use testcontainers::{GenericImage, ImageExt as _, bollard::models::DeviceRequest};
    ///
    /// let device_request = DeviceRequest {
    ///     driver: Some(String::from("nvidia")),
    ///     count: Some(-1), // expose all
    ///     capabilities: Some(vec![vec![String::from("gpu")]]),
    ///     device_ids: None,
    ///     options: None,
    /// };
    ///
    /// let image = GenericImage::new("ubuntu", "24.04")
    ///     .with_device_requests(vec![device_request]);
    /// ```
    #[cfg(feature = "device-requests")]
    fn with_device_requests(self, device_requests: Vec<DeviceRequest>) -> ContainerRequest<I>;

    /// Sets whether to keep stdin open for the container.
    fn with_open_stdin(self, open_stdin: bool) -> ContainerRequest<I>;
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

    fn with_platform(self, platform: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();

        ContainerRequest {
            platform: Some(platform.into()),
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

    fn with_hostname(self, hostname: impl Into<String>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.hostname = Some(hostname.into());
        container_req
    }

    fn with_mount(self, mount: impl Into<Mount>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.mounts.push(mount.into());
        container_req
    }

    fn with_copy_to(
        self,
        target: impl Into<CopyTargetOptions>,
        source: impl Into<CopyDataSource>,
    ) -> ContainerRequest<I> {
        let mut container_req = self.into();
        let target = target.into();
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

    #[cfg(feature = "host-port-exposure")]
    fn with_exposed_host_port(self, port: u16) -> ContainerRequest<I> {
        self.with_exposed_host_ports([port])
    }

    #[cfg(feature = "host-port-exposure")]
    fn with_exposed_host_ports(self, ports: impl IntoIterator<Item = u16>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        let exposures = container_req
            .host_port_exposures
            .get_or_insert_with(Vec::new);

        exposures.extend(ports);
        exposures.sort_unstable();
        exposures.dedup();

        container_req
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

    fn with_host_config_modifier(
        self,
        modifier: impl Fn(&mut HostConfig) + Send + Sync + 'static,
    ) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            host_config_modifier: Some(Arc::new(modifier)),
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

    fn with_user(self, user: impl Into<String>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            user: Some(user.into()),
            ..container_req
        }
    }

    fn with_readonly_rootfs(self, readonly_rootfs: bool) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            readonly_rootfs,
            ..container_req
        }
    }

    fn with_security_opt(self, security_opt: impl Into<String>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req
            .security_opts
            .get_or_insert_with(Vec::new)
            .push(security_opt.into());

        container_req
    }

    fn with_ready_conditions(self, ready_conditions: Vec<WaitFor>) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.ready_conditions = Some(ready_conditions);
        container_req
    }

    fn with_health_check(self, health_check: Healthcheck) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.health_check = Some(health_check);
        container_req
    }

    #[cfg(feature = "device-requests")]
    fn with_device_requests(self, device_requests: Vec<DeviceRequest>) -> ContainerRequest<I> {
        let container_req = self.into();
        ContainerRequest {
            device_requests: Some(device_requests),
            ..container_req
        }
    }

    fn with_open_stdin(self, open_stdin: bool) -> ContainerRequest<I> {
        let mut container_req = self.into();
        container_req.open_stdin = Some(open_stdin);
        container_req
    }
}

#[cfg(all(test, feature = "host-port-exposure"))]
mod tests {
    use super::*;
    use crate::images::generic::GenericImage;

    #[test]
    fn test_with_exposed_host_port_single() {
        let image = GenericImage::new("test", "latest");
        let request = image.with_exposed_host_port(8080);

        assert_eq!(request.host_port_exposures, Some(vec![8080]));
    }

    #[test]
    fn test_with_exposed_host_ports_multiple() {
        let image = GenericImage::new("test", "latest");
        let request = image.with_exposed_host_ports([8080, 9090, 3000]);

        assert_eq!(request.host_port_exposures, Some(vec![3000, 8080, 9090]));
    }

    #[test]
    fn test_with_exposed_host_ports_deduplication() {
        let image = GenericImage::new("test", "latest");
        let request = image.with_exposed_host_ports([8080, 9090, 8080, 3000, 9090]);

        assert_eq!(request.host_port_exposures, Some(vec![3000, 8080, 9090]));
    }

    #[test]
    fn test_with_exposed_host_ports_empty() {
        let image = GenericImage::new("test", "latest");
        let request = image.with_exposed_host_ports([]);

        assert_eq!(request.host_port_exposures, Some(vec![]));
    }

    #[test]
    fn test_with_exposed_host_ports_chaining() {
        let image = GenericImage::new("test", "latest");
        let request = image
            .with_exposed_host_port(8080)
            .with_exposed_host_ports([9090, 3000]);

        assert_eq!(request.host_port_exposures, Some(vec![3000, 8080, 9090]));
    }

    #[test]
    fn test_with_exposed_host_ports_preserves_existing() {
        let image = GenericImage::new("test", "latest");
        let request = image.with_exposed_host_port(8080);

        // The first call already set host_port_exposures to Some(vec![8080])
        // Now we add more ports
        let request = request.with_exposed_host_ports([9090, 3000]);

        // The result should include all ports: 8080 (from first call), 9090, 3000 (from second call)
        assert_eq!(request.host_port_exposures, Some(vec![3000, 8080, 9090]));
    }
}
