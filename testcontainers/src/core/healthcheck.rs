use std::time::Duration;

use bollard::models::HealthConfig;

/// Represents a custom health check configuration for a container.
///
/// This mirrors the options available in Docker's `HEALTHCHECK` instruction,
/// allowing users to define custom health checks at runtime.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// use testcontainers::core::Healthcheck;
///
/// let healthcheck = Healthcheck::cmd_shell("mysqladmin ping -h localhost -u root -proot")
///     .with_interval(Duration::from_secs(2))
///     .with_timeout(Duration::from_secs(1))
///     .with_retries(5)
///     .with_start_period(Duration::from_secs(10));
/// ```
#[derive(Debug, Clone)]
pub struct Healthcheck {
    /// The test command to run.
    test: Vec<String>,
    /// The time to wait between health checks.
    interval: Option<Duration>,
    /// The time to wait before considering the health check failed.
    timeout: Option<Duration>,
    /// The number of consecutive failures needed to consider a container as unhealthy.
    retries: Option<u32>,
    /// Start period for the container to initialize before starting health-retries countdown.
    start_period: Option<Duration>,
    /// The time to wait between health checks during the start period.
    start_interval: Option<Duration>,
}

impl Healthcheck {
    /// Creates a new `Healthcheck` that disables the health check for the container.
    ///
    /// This is equivalent to `HEALTHCHECK NONE` in a Dockerfile.
    pub fn none() -> Self {
        Self {
            test: vec!["NONE".to_string()],
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
        }
    }

    /// Creates a new `Healthcheck` with the specified shell command.
    ///
    /// This is equivalent to `HEALTHCHECK CMD-SHELL <command>` in the Docker API.
    pub fn cmd_shell(command: impl Into<String>) -> Self {
        Self {
            test: vec!["CMD-SHELL".to_string(), command.into()],
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
        }
    }

    /// Creates a new `Healthcheck` with the specified command and arguments.
    ///
    /// This is equivalent to `HEALTHCHECK CMD ["<command>", "<arg1>", ...]` in the Docker API.
    /// The command can be any iterator that yields string-like items.
    pub fn cmd<I, S>(command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut test = vec!["CMD".to_string()];
        test.extend(command.into_iter().map(|s| s.as_ref().to_owned()));
        Self {
            test,
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
        }
    }

    /// Creates an empty healthcheck configuration to customize an image's existing healthcheck.
    ///
    /// This keeps the original healthcheck command from the image, but allows overriding
    /// other parameters like `interval` or `retries`. In the Docker API, this is achieved
    /// by sending an empty `test` field along with the other desired values.
    pub fn empty() -> Self {
        Self {
            test: vec![],
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
        }
    }

    /// Sets the interval between health checks.
    ///
    /// Passing `None` will clear the value and use the Docker default.
    pub fn with_interval(mut self, interval: impl Into<Option<Duration>>) -> Self {
        self.interval = interval.into();
        self
    }

    /// Sets the timeout for each health check.
    ///
    /// Passing `None` will clear the value and use the Docker default.
    pub fn with_timeout(mut self, timeout: impl Into<Option<Duration>>) -> Self {
        self.timeout = timeout.into();
        self
    }

    /// Sets the number of consecutive failures needed to consider the container unhealthy.
    ///
    /// Passing `None` will clear the value and use the Docker default.
    pub fn with_retries(mut self, retries: impl Into<Option<u32>>) -> Self {
        self.retries = retries.into();
        self
    }

    /// Sets the start period for the container to initialize before starting health checks.
    ///
    /// Passing `None` will clear the value and use the Docker default.
    pub fn with_start_period(mut self, start_period: impl Into<Option<Duration>>) -> Self {
        self.start_period = start_period.into();
        self
    }

    /// Sets the interval between health checks during the start period.
    ///
    /// Passing `None` will clear the value and use the Docker default.
    pub fn with_start_interval(mut self, interval: impl Into<Option<Duration>>) -> Self {
        self.start_interval = interval.into();
        self
    }

    /// Returns the test command as a vector of strings.
    pub fn test(&self) -> &[String] {
        &self.test
    }

    /// Returns the interval between health checks.
    pub fn interval(&self) -> Option<Duration> {
        self.interval
    }

    /// Returns the timeout for each health check.
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Returns the number of retries before considering the container unhealthy.
    pub fn retries(&self) -> Option<u32> {
        self.retries
    }

    /// Returns the start period before health checks begin.
    pub fn start_period(&self) -> Option<Duration> {
        self.start_period
    }

    /// Returns the interval between health checks during the start period.
    pub fn start_interval(&self) -> Option<Duration> {
        self.start_interval
    }

    /// Converts this `Healthcheck` into a bollard `HealthConfig` for use with Docker API.
    pub(crate) fn into_health_config(self) -> HealthConfig {
        // Helper to convert Duration to i64 nanoseconds, capping at i64::MAX.
        // Docker interprets 0 as the default value (e.g., 30s for interval).
        // A negative value would disable the healthcheck, but our `Duration` type ensures it's always non-negative.
        let to_nanos = |d: Duration| -> i64 { d.as_nanos().try_into().unwrap_or(i64::MAX) };

        HealthConfig {
            test: Some(self.test),
            interval: self.interval.map(to_nanos),
            timeout: self.timeout.map(to_nanos),
            retries: self.retries.map(|r| r as i64),
            start_period: self.start_period.map(to_nanos),
            start_interval: self.start_interval.map(to_nanos),
        }
    }
}
