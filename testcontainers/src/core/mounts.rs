/// Represents a filesystem mount.
/// For more information see [Docker Storage](https://docs.docker.com/storage/) documentation.
#[derive(Debug, Clone)]
pub struct Mount {
    access_mode: AccessMode,
    mount_type: MountType,
    source: Option<String>,
    target: Option<String>,
    tmpfs_options: Option<MountTmpfsOptions>,
}

/// Options for configuring tmpfs mounts.
#[derive(Debug, Clone, Default)]
pub struct MountTmpfsOptions {
    /// Size of the tmpfs mount in bytes.
    pub size_bytes: Option<i64>,
    /// Permission mode for the tmpfs mount as an integer.
    pub mode: Option<i64>,
}

#[derive(parse_display::Display, Debug, Copy, Clone, PartialEq)]
#[display(style = "snake_case")]
pub enum MountType {
    Bind,
    Volume,
    Tmpfs,
}

#[derive(parse_display::Display, Debug, Copy, Clone, PartialEq)]
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
            tmpfs_options: None,
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
            tmpfs_options: None,
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
            tmpfs_options: None,
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

    /// Configures the size of a tmpfs mount in bytes.
    ///
    /// This method is only applicable to tmpfs mounts and will be ignored for other mount types.
    ///
    /// # Example
    /// ```
    /// # use testcontainers::core::Mount;
    /// let mount = Mount::tmpfs_mount("/tmp")
    ///     .with_size_bytes(20_000_000_000); // 20GB
    /// ```
    pub fn with_size_bytes(mut self, size: i64) -> Self {
        let opts = self
            .tmpfs_options
            .get_or_insert_with(MountTmpfsOptions::default);
        opts.size_bytes = Some(size);
        self
    }

    /// Configures the size of a tmpfs mount using human-readable format.
    ///
    /// This method is only applicable to tmpfs mounts and will be ignored for other mount types.
    ///
    /// Supports the following formats:
    /// - `"100k"` or `"100K"` - kilobytes (1000 bytes)
    /// - `"100m"` or `"100M"` - megabytes (1000^2 bytes)
    /// - `"100g"` or `"100G"` - gigabytes (1000^3 bytes)
    ///
    /// # Example
    /// ```
    /// # use testcontainers::core::Mount;
    /// let mount = Mount::tmpfs_mount("/tmp")
    ///     .with_size("20g"); // 20GB
    /// ```
    ///
    /// # Panics
    /// Panics if the size string cannot be parsed or contains invalid values.
    pub fn with_size(self, size: &str) -> Self {
        let bytes = parse_size(size).expect("Invalid size format");
        self.with_size_bytes(bytes)
    }

    /// Configures the permission mode for a tmpfs mount.
    ///
    /// This method is only applicable to tmpfs mounts and will be ignored for other mount types.
    ///
    /// # Example
    /// ```
    /// # use testcontainers::core::Mount;
    /// let mount = Mount::tmpfs_mount("/tmp")
    ///     .with_mode(0o1777); // sticky bit + rwxrwxrwx
    /// ```
    pub fn with_mode(mut self, mode: i64) -> Self {
        let opts = self
            .tmpfs_options
            .get_or_insert_with(MountTmpfsOptions::default);
        opts.mode = Some(mode);
        self
    }

    /// Returns the tmpfs options if configured.
    pub fn tmpfs_options(&self) -> Option<&MountTmpfsOptions> {
        self.tmpfs_options.as_ref()
    }
}

/// Parses a human-readable size string into bytes.
///
/// Supports: k/K (kilobytes), m/M (megabytes), g/G (gigabytes)
fn parse_size(size: &str) -> Result<i64, String> {
    let size = size.trim();
    if size.is_empty() {
        return Err("Size string is empty".to_string());
    }

    let (number_part, unit) = if let Some(stripped) = size.strip_suffix(['k', 'K']) {
        (stripped, 1_000)
    } else if let Some(stripped) = size.strip_suffix(['m', 'M']) {
        (stripped, 1_000_000)
    } else if let Some(stripped) = size.strip_suffix(['g', 'G']) {
        (stripped, 1_000_000_000)
    } else {
        // No unit specified, treat as bytes
        (size, 1)
    };

    let number: i64 = number_part
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse number '{}': {}", number_part, e))?;

    if number < 0 {
        return Err("Size cannot be negative".to_string());
    }

    number
        .checked_mul(unit)
        .ok_or_else(|| "Size value overflows i64".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_kilobytes() {
        assert_eq!(parse_size("100k").unwrap(), 100_000);
        assert_eq!(parse_size("100K").unwrap(), 100_000);
        assert_eq!(parse_size("1k").unwrap(), 1_000);
        assert_eq!(parse_size("500k").unwrap(), 500_000);
    }

    #[test]
    fn test_parse_size_megabytes() {
        assert_eq!(parse_size("100m").unwrap(), 100_000_000);
        assert_eq!(parse_size("100M").unwrap(), 100_000_000);
        assert_eq!(parse_size("1m").unwrap(), 1_000_000);
        assert_eq!(parse_size("500m").unwrap(), 500_000_000);
    }

    #[test]
    fn test_parse_size_gigabytes() {
        assert_eq!(parse_size("1g").unwrap(), 1_000_000_000);
        assert_eq!(parse_size("1G").unwrap(), 1_000_000_000);
        assert_eq!(parse_size("20g").unwrap(), 20_000_000_000);
        assert_eq!(parse_size("100G").unwrap(), 100_000_000_000);
    }

    #[test]
    fn test_parse_size_bytes() {
        assert_eq!(parse_size("1000").unwrap(), 1000);
        assert_eq!(parse_size("12345").unwrap(), 12345);
        assert_eq!(parse_size("0").unwrap(), 0);
    }

    #[test]
    fn test_parse_size_with_whitespace() {
        assert_eq!(parse_size("  100m  ").unwrap(), 100_000_000);
        assert_eq!(parse_size("1g ").unwrap(), 1_000_000_000);
        assert_eq!(parse_size(" 500k").unwrap(), 500_000);
    }

    #[test]
    fn test_parse_size_empty_string() {
        assert!(parse_size("").is_err());
        assert!(parse_size("   ").is_err());
    }

    #[test]
    fn test_parse_size_invalid_number() {
        assert!(parse_size("abc").is_err());
        assert!(parse_size("12.5g").is_err());
        assert!(parse_size("1.5m").is_err());
        assert!(parse_size("k").is_err());
        assert!(parse_size("m").is_err());
    }

    #[test]
    fn test_parse_size_negative() {
        assert!(parse_size("-100m").is_err());
        assert!(parse_size("-1g").is_err());
    }

    #[test]
    fn test_parse_size_overflow() {
        // i64::MAX is 9,223,372,036,854,775,807
        // This would overflow when multiplied by 1_000_000_000
        assert!(parse_size("10000000000g").is_err());
    }

    #[test]
    fn test_tmpfs_mount_with_size_bytes() {
        let mount = Mount::tmpfs_mount("/tmp").with_size_bytes(1_000_000_000);

        assert_eq!(mount.mount_type(), MountType::Tmpfs);
        assert_eq!(mount.target(), Some("/tmp"));
        assert!(mount.tmpfs_options().is_some());
        assert_eq!(
            mount.tmpfs_options().unwrap().size_bytes,
            Some(1_000_000_000)
        );
    }

    #[test]
    fn test_tmpfs_mount_with_size_string() {
        let mount = Mount::tmpfs_mount("/tmp").with_size("20g");

        assert_eq!(mount.mount_type(), MountType::Tmpfs);
        assert!(mount.tmpfs_options().is_some());
        assert_eq!(
            mount.tmpfs_options().unwrap().size_bytes,
            Some(20_000_000_000)
        );
    }

    #[test]
    fn test_tmpfs_mount_with_mode() {
        let mount = Mount::tmpfs_mount("/tmp").with_mode(0o1777);

        assert_eq!(mount.mount_type(), MountType::Tmpfs);
        assert!(mount.tmpfs_options().is_some());
        assert_eq!(mount.tmpfs_options().unwrap().mode, Some(0o1777));
    }

    #[test]
    fn test_tmpfs_mount_with_size_and_mode() {
        let mount = Mount::tmpfs_mount("/tmp")
            .with_size("10g")
            .with_mode(0o1777);

        assert_eq!(mount.mount_type(), MountType::Tmpfs);
        let opts = mount.tmpfs_options().unwrap();
        assert_eq!(opts.size_bytes, Some(10_000_000_000));
        assert_eq!(opts.mode, Some(0o1777));
    }

    #[test]
    fn test_tmpfs_mount_without_options() {
        let mount = Mount::tmpfs_mount("/tmp");

        assert_eq!(mount.mount_type(), MountType::Tmpfs);
        assert_eq!(mount.target(), Some("/tmp"));
        assert!(mount.tmpfs_options().is_none());
    }

    #[test]
    fn test_bind_mount_has_no_tmpfs_options() {
        let mount = Mount::bind_mount("/host", "/container");

        assert_eq!(mount.mount_type(), MountType::Bind);
        assert!(mount.tmpfs_options().is_none());
    }

    #[test]
    fn test_volume_mount_has_no_tmpfs_options() {
        let mount = Mount::volume_mount("my-vol", "/container");

        assert_eq!(mount.mount_type(), MountType::Volume);
        assert!(mount.tmpfs_options().is_none());
    }

    #[test]
    #[should_panic(expected = "Invalid size format")]
    fn test_with_size_panics_on_invalid_format() {
        Mount::tmpfs_mount("/tmp").with_size("invalid");
    }

    #[test]
    fn test_tmpfs_mount_builder_chain() {
        let mount = Mount::tmpfs_mount("/var/lib/data")
            .with_size_bytes(5_000_000_000)
            .with_mode(0o755)
            .with_access_mode(AccessMode::ReadOnly);

        assert_eq!(mount.mount_type(), MountType::Tmpfs);
        assert_eq!(mount.access_mode(), AccessMode::ReadOnly);
        let opts = mount.tmpfs_options().unwrap();
        assert_eq!(opts.size_bytes, Some(5_000_000_000));
        assert_eq!(opts.mode, Some(0o755));
    }

    #[test]
    fn test_tmpfs_options_can_be_overwritten() {
        let mount = Mount::tmpfs_mount("/tmp").with_size("1g").with_size("2g"); // Override previous size

        let opts = mount.tmpfs_options().unwrap();
        assert_eq!(opts.size_bytes, Some(2_000_000_000));
    }
}
