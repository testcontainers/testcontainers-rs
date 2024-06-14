use std::time::Duration;

use crate::{
    core::{CgroupnsMode, ContainerPort, Host, Mount, PortMapping},
    ContainerRequest, Image,
};

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
    /// let another_runnable_image = image.with_cmd(cmd);
    ///
    /// assert!(another_runnable_image.cmd().eq(overridden_cmd.cmd()));
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

    /// Adds an environment variable to the container.
    fn with_env_var(self, name: impl Into<String>, value: impl Into<String>)
        -> ContainerRequest<I>;

    /// Adds a host to the container.
    fn with_host(self, key: impl Into<String>, value: impl Into<Host>) -> ContainerRequest<I>;

    /// Adds a mount to the container.
    fn with_mount(self, mount: impl Into<Mount>) -> ContainerRequest<I>;

    /// Adds a port mapping to the container, mapping the host port to the container's internal port.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use testcontainers::{GenericImage, ImageExt};
    /// use testcontainers::core::IntoContainerPort;
    ///
    /// let image = GenericImage::new("image", "tag").with_mapped_port(8080, 80.tcp());
    /// ```
    fn with_mapped_port(self, host_port: u16, container_port: ContainerPort) -> ContainerRequest<I>;

    /// Sets the container to run in privileged mode.
    fn with_privileged(self, privileged: bool) -> ContainerRequest<I>;

    /// cgroup namespace mode for the container. Possible values are:
    /// - [`CgroupnsMode::Private`]: the container runs in its own private cgroup namespace
    /// - [`CgroupnsMode::Host`]: use the host system's cgroup namespace
    /// If not specified, the daemon default is used, which can either be `\"private\"` or `\"host\"`, depending on daemon version, kernel support and configuration.
    fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> ContainerRequest<I>;

    /// Sets the usernamespace mode for the container when usernamespace remapping option is enabled.
    fn with_userns_mode(self, userns_mode: &str) -> ContainerRequest<I>;

    /// Sets the shared memory size in bytes
    fn with_shm_size(self, bytes: u64) -> ContainerRequest<I>;

    /// Sets the startup timeout for the container. The default is 60 seconds.
    fn with_startup_timeout(self, timeout: Duration) -> ContainerRequest<I>;
}

/// Implements the [`ImageExt`] trait for the every type that can be converted into a [`ContainerRequest`].
impl<RI: Into<ContainerRequest<I>>, I: Image> ImageExt<I> for RI {
    fn with_cmd(self, cmd: impl IntoIterator<Item = impl Into<String>>) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            overridden_cmd: cmd.into_iter().map(Into::into).collect(),
            ..runnable
        }
    }

    fn with_name(self, name: impl Into<String>) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            image_name: Some(name.into()),
            ..runnable
        }
    }

    fn with_tag(self, tag: impl Into<String>) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            image_tag: Some(tag.into()),
            ..runnable
        }
    }

    fn with_container_name(self, name: impl Into<String>) -> ContainerRequest<I> {
        let runnable = self.into();

        ContainerRequest {
            container_name: Some(name.into()),
            ..runnable
        }
    }

    fn with_network(self, network: impl Into<String>) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            network: Some(network.into()),
            ..runnable
        }
    }

    fn with_env_var(
        self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> ContainerRequest<I> {
        let mut runnable = self.into();
        runnable.env_vars.insert(name.into(), value.into());
        runnable
    }

    fn with_host(self, key: impl Into<String>, value: impl Into<Host>) -> ContainerRequest<I> {
        let mut runnable = self.into();
        runnable.hosts.insert(key.into(), value.into());
        runnable
    }

    fn with_mount(self, mount: impl Into<Mount>) -> ContainerRequest<I> {
        let mut runnable = self.into();
        runnable.mounts.push(mount.into());
        runnable
    }

    fn with_mapped_port(self, host_port: u16, container_port: ContainerPort) -> ContainerRequest<I> {
        let runnable = self.into();
        let mut ports = runnable.ports.unwrap_or_default();
        ports.push(PortMapping::new(host_port, container_port.into()));

        ContainerRequest {
            ports: Some(ports),
            ..runnable
        }
    }

    fn with_privileged(self, privileged: bool) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            privileged,
            ..runnable
        }
    }

    fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            cgroupns_mode: Some(cgroupns_mode),
            ..runnable
        }
    }

    fn with_userns_mode(self, userns_mode: &str) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            userns_mode: Some(String::from(userns_mode)),
            ..runnable
        }
    }

    fn with_shm_size(self, bytes: u64) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            shm_size: Some(bytes),
            ..runnable
        }
    }

    fn with_startup_timeout(self, timeout: Duration) -> ContainerRequest<I> {
        let runnable = self.into();
        ContainerRequest {
            startup_timeout: Some(timeout),
            ..runnable
        }
    }
}
