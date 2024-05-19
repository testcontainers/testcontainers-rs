use std::{fmt, io::BufRead, net::IpAddr};

use crate::{
    core::{env, errors::ExecError, ports::Ports, ExecCommand},
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
///     let container = MyImage::default().start();
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
    runtime: tokio::runtime::Runtime,
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
    pub(crate) fn new(runtime: tokio::runtime::Runtime, async_impl: ContainerAsync<I>) -> Self {
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

    /// Returns a reference to the [`arguments`] of the [`Image`] of this container.
    ///
    /// Access to this is useful to retrieve relevant information which had been passed as [`arguments`]
    ///
    /// [`Image`]: trait.Image.html
    /// [`arguments`]: trait.Image.html#associatedtype.Args
    pub fn image_args(&self) -> &I::Args {
        self.async_impl().image_args()
    }

    pub fn ports(&self) -> Ports {
        self.rt().block_on(self.async_impl().ports())
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv4 interfaces.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will panic.
    ///
    /// # Panics
    ///
    /// This method panics if the given port is not mapped.
    /// Testcontainers is designed to be used in tests only. If a certain port is not mapped, the container
    /// is unlikely to be useful.
    pub fn get_host_port_ipv4(&self, internal_port: u16) -> u16 {
        self.rt()
            .block_on(self.async_impl().get_host_port_ipv4(internal_port))
    }

    /// Returns the mapped host port for an internal port of this docker container, on the host's
    /// IPv6 interfaces.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker container does not expose a port, this method will panic.
    ///
    /// # Panics
    ///
    /// This method panics if the given port is not mapped.
    /// Testcontainers is designed to be used in tests only. If a certain port is not mapped, the container
    /// is unlikely to be useful.
    pub fn get_host_port_ipv6(&self, internal_port: u16) -> u16 {
        self.rt()
            .block_on(self.async_impl().get_host_port_ipv6(internal_port))
    }

    /// Returns the bridge ip address of docker container as specified in NetworkSettings.Networks.IPAddress
    pub fn get_bridge_ip_address(&self) -> IpAddr {
        self.rt()
            .block_on(self.async_impl().get_bridge_ip_address())
    }

    /// Returns the host that this container may be reached on (may not be the local machine)
    /// Suitable for use in URL
    pub fn get_host(&self) -> url::Host {
        self.rt().block_on(self.async_impl().get_host())
    }

    /// Executes a command in the container.
    pub fn exec(&self, cmd: ExecCommand) -> Result<exec::SyncExecResult<'_>, ExecError> {
        let async_exec = self.rt().block_on(self.async_impl().exec(cmd))?;
        Ok(exec::SyncExecResult {
            inner: async_exec,
            runtime: self.rt(),
        })
    }

    /// Stops the container (not the same with `pause`).
    pub fn stop(&self) {
        self.rt().block_on(self.async_impl().stop());
    }

    /// Starts the container.
    pub fn start(&self) {
        self.rt().block_on(self.async_impl().start());
    }

    /// Removes the container.
    pub fn rm(mut self) {
        if let Some(active) = self.inner.take() {
            active.runtime.block_on(active.async_impl.rm());
        }
    }

    /// Returns a reader for stdout.
    pub fn stdout(&self) -> Box<dyn BufRead + '_> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.async_impl().stdout(),
            self.rt(),
        ))
    }

    /// Returns a reader for stderr.
    pub fn stderr(&self) -> Box<dyn BufRead + '_> {
        Box::new(sync_reader::SyncReadBridge::new(
            self.async_impl().stderr(),
            self.rt(),
        ))
    }

    /// Returns reference to inner `Runtime`. It's safe to unwrap because it's `Some` until `Container` is dropped.
    fn rt(&self) -> &tokio::runtime::Runtime {
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
                    env::Command::Remove => active.async_impl.rm().await,
                    env::Command::Keep => {}
                }
            });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{core::WaitFor, runners::SyncRunner, GenericImage, RunnableImage};

    #[derive(Debug, Default)]
    pub struct HelloWorld;

    impl Image for HelloWorld {
        type Args = ();

        fn name(&self) -> String {
            "hello-world".to_owned()
        }

        fn tag(&self) -> String {
            "latest".to_owned()
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
    fn async_logs_are_accessible() {
        let image = GenericImage::new("testcontainers/helloworld", "1.1.0");
        let container = RunnableImage::from(image).start();

        let mut stderr_lines = container.stderr().lines();

        let expected_messages = [
            "DELAY_START_MSEC: 0",
            "Sleeping for 0 ms",
            "Starting server on port 8080",
            "Sleeping for 0 ms",
            "Starting server on port 8081",
            "Ready, listening on 8080 and 8081",
        ];
        for expected_message in expected_messages {
            let line = stderr_lines.next().unwrap().unwrap();
            assert!(
                line.contains(expected_message),
                "Log message ('{line}') doesn't contain expected message ('{expected_message}')"
            );
        }

        // logs are accessible after container is stopped
        container.stop();

        // stdout is empty
        let mut stdout = String::new();
        container.stdout().read_to_string(&mut stdout).unwrap();
        assert_eq!(stdout, "");
        // stderr contains 6 lines
        let mut stderr = String::new();
        container.stderr().read_to_string(&mut stderr).unwrap();
        assert_eq!(
            stderr.lines().count(),
            6,
            "unexpected stderr size: {}",
            stderr
        );
    }
}
