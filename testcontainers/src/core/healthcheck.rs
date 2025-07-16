use std::time::Duration;

use bollard_stubs::models::HealthConfig;

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
        S: Into<String>,
    {
        let mut test = vec!["CMD".to_string()];
        test.extend(command.into_iter().map(Into::into));
        Self {
            test,
            interval: None,
            timeout: None,
            retries: None,
            start_period: None,
            start_interval: None,
        }
    }

    /// Inherits the health check from the image's configuration.
    ///
    /// This allows for overriding parts of the health check configuration
    /// (e.g., interval, retries) while keeping the test command from the image.
    /// This is represented by an empty `Test` field in the Docker API.
    pub fn inherit() -> Self {
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
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Sets the timeout for each health check.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the number of consecutive failures needed to consider the container unhealthy.
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }

    /// Sets the start period for the container to initialize before starting health checks.
    pub fn with_start_period(mut self, start_period: Duration) -> Self {
        self.start_period = Some(start_period);
        self
    }

    /// Sets the interval between health checks during the start period.
    pub fn with_start_interval(mut self, interval: Duration) -> Self {
        self.start_interval = Some(interval);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthcheck_cmd_shell() {
        let healthcheck = Healthcheck::cmd_shell("curl -f http://localhost:8080/health");
        assert_eq!(
            healthcheck.test(),
            &["CMD-SHELL", "curl -f http://localhost:8080/health"]
        );
    }

    #[test]
    fn test_healthcheck_cmd() {
        let healthcheck = Healthcheck::cmd(&["curl", "-f", "http://localhost:8080/health"]);
        assert_eq!(
            healthcheck.test(),
            &["CMD", "curl", "-f", "http://localhost:8080/health"]
        );
    }

    #[test]
    fn test_healthcheck_cmd_with_vec_string() {
        let cmd_vec = vec![
            "curl".to_string(),
            "-f".to_string(),
            "http://localhost:8080/health".to_string(),
        ];
        let healthcheck = Healthcheck::cmd(cmd_vec);
        assert_eq!(
            healthcheck.test(),
            &["CMD", "curl", "-f", "http://localhost:8080/health"]
        );
    }

    #[test]
    fn test_healthcheck_builder_pattern() {
        let healthcheck = Healthcheck::cmd_shell("mysql ping")
            .with_interval(Duration::from_secs(5))
            .with_timeout(Duration::from_secs(3))
            .with_retries(4)
            .with_start_period(Duration::from_secs(15))
            .with_start_interval(Duration::from_secs(2));

        assert_eq!(healthcheck.interval(), Some(Duration::from_secs(5)));
        assert_eq!(healthcheck.timeout(), Some(Duration::from_secs(3)));
        assert_eq!(healthcheck.retries(), Some(4));
        assert_eq!(healthcheck.start_period(), Some(Duration::from_secs(15)));
        assert_eq!(healthcheck.start_interval(), Some(Duration::from_secs(2)));
    }

    #[test]
    fn test_healthcheck_into_health_config() {
        let healthcheck = Healthcheck::cmd_shell("curl -f http://localhost/health")
            .with_interval(Duration::from_secs(30))
            .with_timeout(Duration::from_secs(5))
            .with_retries(3)
            .with_start_period(Duration::from_secs(10))
            .with_start_interval(Duration::from_secs(2));

        let health_config = healthcheck.into_health_config();

        assert_eq!(
            health_config.test,
            Some(vec![
                "CMD-SHELL".to_string(),
                "curl -f http://localhost/health".to_string()
            ])
        );
        assert_eq!(health_config.interval, Some(30_000_000_000)); // 30 seconds in nanoseconds
        assert_eq!(health_config.timeout, Some(5_000_000_000)); // 5 seconds in nanoseconds
        assert_eq!(health_config.retries, Some(3));
        assert_eq!(health_config.start_period, Some(10_000_000_000)); // 10 seconds in nanoseconds
        assert_eq!(health_config.start_interval, Some(2_000_000_000));
    }

    #[test]
    fn test_healthcheck_none() {
        let healthcheck = Healthcheck::none();
        assert_eq!(healthcheck.test(), &["NONE"]);
        assert_eq!(healthcheck.interval(), None);
        assert_eq!(healthcheck.timeout(), None);
        assert_eq!(healthcheck.retries(), None);
        assert_eq!(healthcheck.start_period(), None);
        assert_eq!(healthcheck.start_interval(), None);
    }

    #[test]
    fn test_healthcheck_none_into_health_config() {
        let healthcheck = Healthcheck::none();
        let health_config = healthcheck.into_health_config();

        assert_eq!(health_config.test, Some(vec!["NONE".to_string()]));
        assert_eq!(health_config.interval, None);
        assert_eq!(health_config.timeout, None);
        assert_eq!(health_config.retries, None);
        assert_eq!(health_config.start_period, None);
        assert_eq!(health_config.start_interval, None);
    }

    #[test]
    fn test_duration_overflow_into_health_config() {
        let very_long_duration = Duration::from_nanos(i64::MAX as u64) + Duration::from_nanos(1);
        let healthcheck = Healthcheck::cmd_shell("check").with_interval(very_long_duration);
        let health_config = healthcheck.into_health_config();
        assert_eq!(health_config.interval, Some(i64::MAX));
    }

    #[test]
    fn test_healthcheck_inherit() {
        let healthcheck = Healthcheck::inherit();
        assert!(healthcheck.test().is_empty());
        assert_eq!(healthcheck.interval(), None);
        assert_eq!(healthcheck.timeout(), None);
        assert_eq!(healthcheck.retries(), None);
        assert_eq!(healthcheck.start_period(), None);
        assert_eq!(healthcheck.start_interval(), None);
    }

    #[test]
    fn test_healthcheck_inherit_into_health_config() {
        let healthcheck = Healthcheck::inherit()
            .with_interval(Duration::from_secs(1))
            .with_retries(10);
        let health_config = healthcheck.into_health_config();

        assert_eq!(health_config.test, Some(vec![]));
        assert_eq!(health_config.interval, Some(1_000_000_000));
        assert_eq!(health_config.retries, Some(10));
        assert_eq!(health_config.timeout, None);
        assert_eq!(health_config.start_period, None);
        assert_eq!(health_config.start_interval, None);
    }
}
