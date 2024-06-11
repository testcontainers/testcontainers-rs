use std::time::Duration;

use crate::{
    core::{CgroupnsMode, Host, Mount, PortMapping},
    Image, RunnableImage,
};

pub trait ImageExt<I: Image> {
    fn with_cmd(self, cmd: impl IntoIterator<Item = impl Into<String>>) -> RunnableImage<I>;
    fn with_name(self, name: impl Into<String>) -> RunnableImage<I>;
    fn with_tag(self, tag: impl Into<String>) -> RunnableImage<I>;
    fn with_container_name(self, name: impl Into<String>) -> RunnableImage<I>;
    fn with_network(self, network: impl Into<String>) -> RunnableImage<I>;
    fn with_env_var(self, name: impl Into<String>, value: impl Into<String>) -> RunnableImage<I>;
    fn with_host(self, key: impl Into<String>, value: impl Into<Host>) -> RunnableImage<I>;
    fn with_mount(self, mount: impl Into<Mount>) -> RunnableImage<I>;
    fn with_mapped_port<P: Into<PortMapping>>(self, port: P) -> RunnableImage<I>;
    fn with_privileged(self, privileged: bool) -> RunnableImage<I>;
    fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> RunnableImage<I>;
    fn with_userns_mode(self, userns_mode: &str) -> RunnableImage<I>;
    fn with_shm_size(self, bytes: u64) -> RunnableImage<I>;
    fn with_startup_timeout(self, timeout: Duration) -> RunnableImage<I>;
}

impl<RI: Into<RunnableImage<I>>, I: Image> ImageExt<I> for RI {
    /// Returns a new RunnableImage with the specified (overridden) `CMD` ([`Image::cmd`]).
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
    fn with_cmd(self, cmd: impl IntoIterator<Item = impl Into<String>>) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            overridden_cmd: cmd.into_iter().map(Into::into).collect(),
            ..runnable
        }
    }

    /// Overrides the fully qualified image name (consists of `{domain}/{owner}/{image}`).
    /// Can be used to specify a custom registry or owner.
    fn with_name(self, name: impl Into<String>) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            image_name: Some(name.into()),
            ..runnable
        }
    }

    /// Overrides the image tag.
    ///
    /// There is no guarantee that the specified tag for an image would result in a
    /// running container. Users of this API are advised to use this at their own risk.
    fn with_tag(self, tag: impl Into<String>) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            image_tag: Some(tag.into()),
            ..runnable
        }
    }

    /// Sets the container name.
    fn with_container_name(self, name: impl Into<String>) -> RunnableImage<I> {
        let runnable = self.into();

        RunnableImage {
            container_name: Some(name.into()),
            ..runnable
        }
    }

    /// Sets the network the container will be connected to.
    fn with_network(self, network: impl Into<String>) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            network: Some(network.into()),
            ..runnable
        }
    }

    /// Adds an environment variable to the container.
    fn with_env_var(self, name: impl Into<String>, value: impl Into<String>) -> RunnableImage<I> {
        let mut runnable = self.into();
        runnable.env_vars.insert(name.into(), value.into());
        runnable
    }

    /// Adds a host to the container.
    fn with_host(self, key: impl Into<String>, value: impl Into<Host>) -> RunnableImage<I> {
        let mut runnable = self.into();
        runnable.hosts.insert(key.into(), value.into());
        runnable
    }

    /// Adds a mount to the container.
    fn with_mount(self, mount: impl Into<Mount>) -> RunnableImage<I> {
        let mut runnable = self.into();
        runnable.mounts.push(mount.into());
        runnable
    }

    /// Adds a port mapping to the container.
    fn with_mapped_port<P: Into<PortMapping>>(self, port: P) -> RunnableImage<I> {
        let runnable = self.into();
        let mut ports = runnable.ports.unwrap_or_default();
        ports.push(port.into());

        RunnableImage {
            ports: Some(ports),
            ..runnable
        }
    }

    /// Sets the container to run in privileged mode.
    fn with_privileged(self, privileged: bool) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            privileged,
            ..runnable
        }
    }

    /// cgroup namespace mode for the container. Possible values are:
    /// - `\"private\"`: the container runs in its own private cgroup namespace
    /// - `\"host\"`: use the host system's cgroup namespace
    /// If not specified, the daemon default is used, which can either be `\"private\"` or `\"host\"`, depending on daemon version, kernel support and configuration.
    fn with_cgroupns_mode(self, cgroupns_mode: CgroupnsMode) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            cgroupns_mode: Some(cgroupns_mode),
            ..runnable
        }
    }

    /// Sets the usernamespace mode for the container when usernamespace remapping option is enabled.
    fn with_userns_mode(self, userns_mode: &str) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            userns_mode: Some(String::from(userns_mode)),
            ..runnable
        }
    }

    /// Sets the shared memory size in bytes
    fn with_shm_size(self, bytes: u64) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            shm_size: Some(bytes),
            ..runnable
        }
    }

    /// Sets the startup timeout for the container. The default is 60 seconds.
    fn with_startup_timeout(self, timeout: Duration) -> RunnableImage<I> {
        let runnable = self.into();
        RunnableImage {
            startup_timeout: Some(timeout),
            ..runnable
        }
    }
}
