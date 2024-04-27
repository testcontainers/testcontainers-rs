/// Represents a filesystem mount.
/// For more information see [Docker Storage](https://docs.docker.com/storage/) documentation.
#[derive(Debug, Clone)]
pub struct Mount {
    access_mode: AccessMode,
    mount_type: MountType,
    source: Option<String>,
    target: Option<String>,
}

#[derive(parse_display::Display, Debug, Copy, Clone)]
#[display(style = "snake_case")]
pub enum MountType {
    Bind,
    Volume,
    Tmpfs,
}

#[derive(parse_display::Display, Debug, Copy, Clone)]
pub enum AccessMode {
    #[display("ro")]
    ReadOnly,
    #[display("rw")]
    ReadWrite,
}

impl Mount {
    /// Creates a `bind-mount`.
    /// Can be used to mount a file or directory on the host system into a container.
    ///
    /// See [bind-mounts documentation](https://docs.docker.com/storage/bind-mounts/) for more information.
    pub fn bind_mount(host_path: impl Into<String>, container_path: impl Into<String>) -> Self {
        Self {
            access_mode: AccessMode::ReadWrite,
            mount_type: MountType::Bind,
            source: Some(host_path.into()),
            target: Some(container_path.into()),
        }
    }

    /// Creates a named `volume`.
    /// Can be used to share data between containers or persist data on the host system.
    /// The volume isn't removed when the container is removed.
    ///
    /// See [volumes documentation](https://docs.docker.com/storage/volumes/) for more information.
    pub fn volume_mount(name: impl Into<String>, container_path: impl Into<String>) -> Self {
        Self {
            access_mode: AccessMode::ReadWrite,
            mount_type: MountType::Volume,
            source: Some(name.into()),
            target: Some(container_path.into()),
        }
    }

    /// Creates a `tmpfs` mount.
    /// Can be used to mount a temporary filesystem in the container's memory.
    /// `tmpfs` mount is removed when the container is removed.
    ///
    /// See [tmpfs documentation](https://docs.docker.com/storage/tmpfs/) for more information.
    pub fn tmpfs_mount(container_path: impl Into<String>) -> Self {
        Self {
            access_mode: AccessMode::ReadWrite,
            mount_type: MountType::Tmpfs,
            source: None,
            target: Some(container_path.into()),
        }
    }

    /// Sets the access mode for the mount.
    /// Default is `AccessMode::ReadWrite`.
    pub fn with_access_mode(mut self, access_mode: AccessMode) -> Self {
        self.access_mode = access_mode;
        self
    }

    /// Docker mount access mode.
    pub fn access_mode(&self) -> AccessMode {
        self.access_mode
    }

    /// Docker mount type.
    pub fn mount_type(&self) -> MountType {
        self.mount_type
    }

    /// Absolute path of a file, a directory or volume to mount on the host system.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Absolute path of a file or directory to mount in the container.
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }
}
