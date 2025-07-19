use std::{fmt, io::BufRead, net::IpAddr, sync::Arc};

use crate::{
    core::{env, error::Result, ports::Ports, ContainerPort, ExecCommand},
    ContainerAsync, Image,
};

pub(super) mod exec;
mod sync_reader;

/// Represents a running docker container.
///
/// Containers have a [`custom destructor`][drop_impl] that removes them as soon as they go out of scope:
///
/// ```rust,no_run
/// use testcontainers::*;
/// #[test]
/// fn a_test() {
///     let container = MyImage::default().start().unwrap();
///     // Docker container is stopped/removed at the end of this scope.
/// }
/// ```
///
/// [drop_impl]: struct.Container.html#impl-Drop
pub struct Container<I: Image> {
    inner: Option<ActiveContainer<I>>,
}

/// Internal representation of a running docker container, to be able to terminate runtime correctly when `Container` is dropped.
struct ActiveContainer<I: Image> {
    runtime: Arc<tokio::runtime::Runtime>,
    async_impl: ContainerAsync<I>,
}

impl<I> fmt::Debug for Container<I>
where
    I: fmt::Debug + Image,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Container")
            .field("id", &self.id())
            .field("image", &self.image())
            .field("ports", &self.ports())
            .field("command", &self.async_impl().docker_client.config.command())
            .finish()
    }
}

impl<I: Image> Container<I> {
    pub(crate) fn new(
        runtime: Arc<tokio::runtime::Runtime>,
        async_impl: ContainerAsync<I>,
    ) -> Self {
        Self {
            inner: Some(ActiveContainer {
                runtime,
                async_impl,
            }),
        }
    }
}

impl<I> Container<I>
where
    I: Image,
{
    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        self.async_impl().id()
    }

    /// Returns a reference to the [`Image`] of this container.
    ///
    /// [`Image`]: trait.Image.html
    pub fn image(&self) -> &I {
        self.async_impl().image()
    }

    pub fn ports(&self) -> Result<Ports> {
        self.rt().block_on(self.async_impl().ports())
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv4 interfaces.
    ///
    /// By default, `u16` is considered as TCP port. Also, you can convert `u16` to [`ContainerPort`] port
    /// by using [`crate::core::IntoContainerPort`] trait.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method returns an error.
    pub fn get_host_port_ipv4(&self, internal_port: impl Into<ContainerPort>) -> Result<u16> {
        self.rt()
            .block_on(self.async_impl().get_host_port_ipv4(internal_port))
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv6 interfaces.
    ///
    /// By default, `u16` is considered as TCP port. Also, you can convert `u16` to [`ContainerPort`] port
    /// by using [`crate::core::IntoContainerPort`] trait.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method returns an error.
    pub fn get_host_port_ipv6(&self, internal_port: impl Into<ContainerPort>) -> Result<u16> {
        self.rt()
            .block_on(self.async_impl().get_host_port_ipv6(internal_port))
    }

    /// Returns the bridge ip address of docker container as specified in NetworkSettings.Networks.IPAddress
    pub fn get_bridge_ip_address(&self) -> Result<IpAddr> {
        self.rt()
            .block_on(self.async_impl().get_bridge_ip_address())
    }

    /// Returns the host that this container may be reached on (may not be the local machine)
    /// Suitable for use in URL
    pub fn get_host(&self) -> Result<url::Host> {
        self.rt().block_on(self.async_impl().get_host())
    }

    /// Executes a command in the container.
    pub fn exec(&self, cmd: ExecCommand) -> Result<exec::SyncExecResult> {
        let async_exec = self.rt().block_on(self.async_impl().exec(cmd))?;
        Ok(exec::SyncExecResult {
            inner: async_exec,
            runtime: self.rt().clone(),
        })
    }

    /// Stops the container (not the same with `pause`) using the default 10 second timeout.
    pub fn stop(&self) -> Result<()> {
        self.rt().block_on(self.async_impl().stop())
    }

    /// Stops the container with timeout before issuing SIGKILL (not the same with `pause`).
    ///
    /// Set Some(-1) to wait indefinitely, None to use system configured default and Some(0)
    /// to forcibly stop the container immediately - otherwise the runtime will issue SIGINT
    /// and then wait timeout_seconds seconds for the process to stop before issuing SIGKILL.
    pub fn stop_with_timeout(&self, timeout_seconds: Option<i32>) -> Result<()> {
        self.rt()
            .block_on(self.async_impl().stop_with_timeout(timeout_seconds))
    }

    /// Starts the container.
    pub fn start(&self) -> Result<()> {
        self.rt().block_on(self.async_impl().start())
    }

    /// Pause the container.
    /// [Docker Engine API](https://docs.docker.com/reference/api/engine/version/v1.48/#tag/Container/operation/ContainerPause)
    pub async fn pause(&self) -> Result<()> {
        self.rt().block_on(self.async_impl().pause())
    }

    /// Resume/Unpause the container.
    /// [Docker Engine API](https://docs.docker.com/reference/api/engine/version/v1.48/#tag/Container/operation/ContainerUnpause)
    pub async fn unpause(&self) -> Result<()> {
        self.rt().block_on(self.async_impl().unpause())
    }

    /// Removes the container.
    pub fn rm(mut self) -> Result<()> {
        if let Some(active) = self.inner.take() {
            active.runtime.block_on(active.async_impl.rm())?;
        }
        Ok(())
    }

    /// Returns a reader for stdout.
    ///
    /// Accepts a boolean parameter to follow the logs:
    ///   - pass `true` to read logs from the moment the container starts until it stops (returns I/O error with kind `UnexpectedEof` if container removed).
    ///   - pass `false` to read logs from startup to present.
    pub fn stdout(&self, follow: bool) -> Box<dyn BufRead + Send> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.async_impl().stdout(follow),
            self.rt().clone(),
        ))
    }

    /// Returns a reader for stderr.
    ///
    /// Accepts a boolean parameter to follow the logs:
    ///   - pass `true` to read logs from the moment the container starts until it stops (returns I/O error with kind `UnexpectedEof` if container removed).
    ///   - pass `false` to read logs from startup to present.
    pub fn stderr(&self, follow: bool) -> Box<dyn BufRead + Send> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.async_impl().stderr(follow),
            self.rt().clone(),
        ))
    }

    /// Returns stdout as a vector of bytes available at the moment of call (from container startup to present).
    ///
    /// If you want to read stdout in chunks, use [`Container::stdout`] instead.
    pub fn stdout_to_vec(&self) -> Result<Vec<u8>> {
        let mut stdout = Vec::new();
        self.stdout(false).read_to_end(&mut stdout)?;
        Ok(stdout)
    }

    /// Returns stderr as a vector of bytes available at the moment of call (from container startup to present).
    ///
    /// If you want to read stderr in chunks, use [`Container::stderr`] instead.
    pub fn stderr_to_vec(&self) -> Result<Vec<u8>> {
        let mut stderr = Vec::new();
        self.stderr(false).read_to_end(&mut stderr)?;
        Ok(stderr)
    }

    /// Returns whether the container is still running.
    pub fn is_running(&self) -> Result<bool> {
        self.rt().block_on(self.async_impl().is_running())
    }

    /// Returns `Some(exit_code)` when the container is finished and `None` when the container is still running.
    pub fn exit_code(&self) -> Result<Option<i64>> {
        self.rt().block_on(self.async_impl().exit_code())
    }

    /// Returns reference to inner `Runtime`. It's safe to unwrap because it's `Some` until `Container` is dropped.
    fn rt(&self) -> &Arc<tokio::runtime::Runtime> {
        &self.inner.as_ref().unwrap().runtime
    }

    /// Returns reference to inner `ContainerAsync`. It's safe to unwrap because it's `Some` until `Container` is dropped.
    fn async_impl(&self) -> &ContainerAsync<I> {
        &self.inner.as_ref().unwrap().async_impl
    }
}

impl<I: Image> Drop for Container<I> {
    fn drop(&mut self) {
        if let Some(active) = self.inner.take() {
            active.runtime.block_on(async {
                match active.async_impl.docker_client.config.command() {
                    env::Command::Remove => {
                        if let Err(e) = active.async_impl.rm().await {
                            log::error!("Failed to remove container on drop: {}", e);
                        }
                    }
                    env::Command::Keep => {}
                }
            });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{core::WaitFor, runners::SyncRunner, GenericImage, ImageExt};

    #[derive(Debug, Default)]
    pub struct HelloWorld;

    impl Image for HelloWorld {
        fn name(&self) -> &str {
            "hello-world"
        }

        fn tag(&self) -> &str {
            "latest"
        }

        fn ready_conditions(&self) -> Vec<WaitFor> {
            vec![WaitFor::message_on_stdout("Hello from Docker!")]
        }
    }

    #[test]
    fn container_should_be_send_and_sync() {
        assert_send_and_sync::<Container<HelloWorld>>();
    }

    fn assert_send_and_sync<T: Send + Sync>() {}

    #[test]
    fn sync_custom_healthcheck_is_applied() -> anyhow::Result<()> {
        use std::time::Duration;

        use crate::core::Healthcheck;

        let healthcheck = Healthcheck::cmd_shell("test -f /etc/passwd")
            .with_interval(Duration::from_secs(1))
            .with_timeout(Duration::from_secs(1))
            .with_retries(2);

        let container = GenericImage::new("alpine", "latest")
            .with_cmd(["sleep", "30"])
            .with_health_check(healthcheck)
            .with_ready_conditions(vec![WaitFor::healthcheck()])
            .start()?;

        let inspect_info = container
            .rt()
            .block_on(container.async_impl().docker_client.inspect(container.id()))?;

        assert!(inspect_info.config.is_some());
        let config = inspect_info
            .config
            .expect("Container config must be present");
        assert!(config.healthcheck.is_some());

        let healthcheck_config = config
            .healthcheck
            .expect("Healthcheck config must be present");
        assert_eq!(
            healthcheck_config.test,
            Some(vec![
                "CMD-SHELL".to_string(),
                "test -f /etc/passwd".to_string()
            ])
        );
        assert_eq!(healthcheck_config.interval, Some(1_000_000_000));
        assert_eq!(healthcheck_config.timeout, Some(1_000_000_000));
        assert_eq!(healthcheck_config.retries, Some(2));
        assert_eq!(healthcheck_config.start_period, None);

        assert!(container.is_running()?);
        Ok(())
    }

    #[test]
    fn sync_logs_are_accessible() -> anyhow::Result<()> {
        let image = GenericImage::new("testcontainers/helloworld", "1.2.0");
        let container = image.start()?;

        let stderr = container.stderr(true);

        // it's possible to send logs to another thread
        let log_follower_thread = std::thread::spawn(move || {
            let mut stderr_lines = stderr.lines();
            let expected_messages = [
                "DELAY_START_MSEC: 0",
                "Sleeping for 0 ms",
                "Starting server on port 8080",
                "Sleeping for 0 ms",
                "Starting server on port 8081",
                "Ready, listening on 8080 and 8081",
            ];
            for expected_message in expected_messages {
                let line = stderr_lines.next().expect("line must exist")?;
                if !line.contains(expected_message) {
                    anyhow::bail!(
                        "Log message ('{}') doesn't contain expected message ('{}')",
                        line,
                        expected_message
                    );
                }
            }
            Ok(())
        });
        log_follower_thread
            .join()
            .map_err(|_| anyhow::anyhow!("failed to join log follower thread"))??;

        // logs are accessible after container is stopped
        container.stop()?;

        // stdout is empty
        let stdout = String::from_utf8(container.stdout_to_vec()?)?;
        assert_eq!(stdout, "");
        // stderr contains 6 lines
        let stderr = String::from_utf8(container.stderr_to_vec()?)?;
        assert_eq!(
            stderr.lines().count(),
            6,
            "unexpected stderr size: {}",
            stderr
        );
        Ok(())
    }
}
