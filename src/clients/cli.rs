use crate::{
    core::{
        env, env::GetEnvValue, logs::LogStream, ports::Ports, Container, Docker, RunnableImage,
    },
    Image,
};
use shiplift::rep::ContainerDetails;
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    process::{Command, Stdio},
    sync::{Arc, RwLock},
    thread::sleep,
    time::{Duration, Instant},
};

const ONE_SECOND: Duration = Duration::from_secs(1);
const ZERO: Duration = Duration::from_secs(0);

/// Implementation of the Docker client API using the docker cli.
///
/// This (fairly naive) implementation of the Docker client API simply creates `Command`s to the `docker` CLI. It thereby assumes that the `docker` CLI is installed and that it is in the PATH of the current execution environment.
#[derive(Debug)]
pub struct Cli {
    inner: Arc<Client>,
}

impl Cli {
    pub fn run<I>(&self, image: impl Into<RunnableImage<I>>) -> Container<'_, I>
    where
        I: Image,
    {
        let runnable = image.into();

        if let Some(network) = runnable.network.as_ref() {
            if self.inner.create_network_if_not_exists(network) {
                let mut guard = self
                    .inner
                    .created_networks
                    .write()
                    .expect("failed to lock RwLock");

                guard.push(network.to_owned());
            }
        }

        let mut command = build_run_command(self.inner.command(), &runnable);

        log::debug!("Executing command: {:?}", &command);

        let output = command.output().expect("Failed to execute docker command");

        assert!(output.status.success(), "failed to start container");
        let container_id = String::from_utf8(output.stdout)
            .expect("output is not valid utf8")
            .trim()
            .to_string();
        self.inner.register_container_started(container_id.clone());

        let client = Cli {
            inner: self.inner.clone(),
        };

        Container::new(container_id, client, runnable, self.inner.command)
    }
}

#[derive(Debug)]
struct Client {
    /// The docker CLI has an issue that if you request logs for a container
    /// too quickly after it was started up, the resulting stream will never
    /// emit any data, even if the container is already emitting logs.
    ///
    /// We keep track of when we started a container in order to make sure
    /// that we wait at least one second after that. Subsequent invocations
    /// directly fetch the logs of a container.
    container_startup_timestamps: RwLock<HashMap<String, Instant>>,
    created_networks: RwLock<Vec<String>>,
    binary: OsString,
    command: env::Command,
}

impl Client {
    fn command(&self) -> Command {
        Command::new(self.binary.clone())
    }

    fn register_container_started(&self, id: String) {
        let mut lock_guard = match self.container_startup_timestamps.write() {
            Ok(lock_guard) => lock_guard,

            // We only need the mutex to not require a &mut self in this function.
            // Data cannot be in-consistent even if a thread panics while holding the lock
            Err(e) => e.into_inner(),
        };
        let start_timestamp = Instant::now();

        log::trace!(
            "Registering starting of container {} at {:?}",
            id,
            start_timestamp
        );

        lock_guard.insert(id, start_timestamp);
    }

    fn time_since_container_was_started(&self, id: &str) -> Option<Duration> {
        let lock_guard = match self.container_startup_timestamps.read() {
            Ok(lock_guard) => lock_guard,

            // We only need the mutex to not require a &mut self in this function.
            // Data cannot be in-consistent even if a thread panics while holding the lock
            Err(e) => e.into_inner(),
        };

        let result = lock_guard.get(id).map(|i| Instant::now() - *i);

        log::trace!("Time since container {} was started: {:?}", id, result);

        result
    }

    fn wait_at_least_one_second_after_container_was_started(&self, id: &str) {
        if let Some(duration) = self.time_since_container_was_started(id) {
            if duration < ONE_SECOND {
                sleep(ONE_SECOND.checked_sub(duration).unwrap_or(ZERO))
            }
        }
    }

    fn create_network_if_not_exists(&self, name: &str) -> bool {
        if self.network_exists(name) {
            return false;
        }

        let mut docker = self.command();
        docker.args(&["network", "create", name]);

        let output = docker.output().expect("failed to create docker network");
        assert!(output.status.success(), "failed to create docker network");

        true
    }

    fn network_exists(&self, name: &str) -> bool {
        let mut docker = self.command();
        docker.args(&["network", "ls", "--format", "{{.Name}}"]);

        let output = docker.output().expect("failed to list docker networks");
        let output = String::from_utf8(output.stdout).expect("output is not valid utf-8");

        output.lines().any(|network| network == name)
    }

    fn delete_networks<I, S>(&self, networks: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut docker = self.command();
        docker.args(&["network", "rm"]);
        docker.args(networks);

        let output = docker.output().expect("failed to delete docker networks");

        assert!(
            output.status.success(),
            "failed to delete docker networks: {}",
            String::from_utf8(output.stderr).unwrap()
        )
    }
}

fn build_run_command<I: Image>(mut command: Command, runnable: &RunnableImage<I>) -> Command {
    let command_ref = &mut command;

    command_ref.arg("run");

    if let Some(network) = runnable.network.as_ref() {
        command_ref.arg(format!("--network={}", network));
    }

    if let Some(name) = runnable.name.as_ref() {
        command_ref.arg(format!("--name={}", name));
    }

    for (key, value) in runnable.image.env_vars() {
        command_ref.arg("-e").arg(format!("{}={}", key, value));
    }

    for (orig, dest) in runnable.image.volumes() {
        command_ref.arg("-v").arg(format!("{}:{}", orig, dest));
    }

    if let Some(entrypoint) = runnable.image.entrypoint() {
        command_ref.arg("--entrypoint").arg(entrypoint);
    }

    if let Some(ports) = runnable.ports.as_ref() {
        for port in ports {
            command_ref
                .arg("-p")
                .arg(format!("{}:{}", port.local, port.internal));
        }
    } else {
        command_ref.arg("-P"); // expose all ports
    }

    command_ref
        .arg("-d") // Always run detached
        .arg(runnable.image.descriptor())
        .args(runnable.image_args.clone().into_iter())
        .stdout(Stdio::piped());

    command
}

impl Default for Cli {
    fn default() -> Self {
        Self::docker()
    }
}

impl Cli {
    /// Create a new client, using the `docker` binary.
    pub fn docker() -> Self {
        Self::new::<env::Os, _>("docker")
    }

    /// Create a new client, using the `podman` binary.
    pub fn podman() -> Self {
        Self::new::<env::Os, _>("podman")
    }

    fn new<E, S>(binary: S) -> Self
    where
        S: Into<OsString>,
        E: GetEnvValue,
    {
        Self {
            inner: Arc::new(Client {
                container_startup_timestamps: Default::default(),
                created_networks: Default::default(),
                binary: binary.into(),
                command: env::command::<E>().unwrap_or_default(),
            }),
        }
    }
}

impl Docker for Cli {
    fn stdout_logs(&self, id: &str) -> LogStream {
        self.inner
            .wait_at_least_one_second_after_container_was_started(id);

        let child = self
            .inner
            .command()
            .arg("logs")
            .arg("-f")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        LogStream::new(child.stdout.expect("stdout to be captured"))
    }

    fn stderr_logs(&self, id: &str) -> LogStream {
        self.inner
            .wait_at_least_one_second_after_container_was_started(id);

        let child = self
            .inner
            .command()
            .arg("logs")
            .arg("-f")
            .arg(id)
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        LogStream::new(child.stderr.expect("stderr to be captured"))
    }

    fn ports(&self, id: &str) -> Ports {
        self.inspect(id)
            .network_settings
            .ports
            .map(Ports::new)
            .unwrap_or_default()
    }

    fn inspect(&self, id: &str) -> ContainerDetails {
        let child = self
            .inner
            .command()
            .arg("inspect")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();

        let mut infos: Vec<ContainerDetails> = serde_json::from_reader(stdout).unwrap();

        let info = infos.remove(0);

        log::trace!("Fetched container info: {:#?}", info);
        info
    }

    fn rm(&self, id: &str) {
        let output = self
            .inner
            .command()
            .arg("rm")
            .arg("-f")
            .arg("-v") // Also remove volumes
            .arg(id)
            .output()
            .expect("Failed to execute docker command");
        let error_msg = "Failed to remove docker container";
        assert!(output.status.success(), "{}", error_msg);
        // The container's id is printed on stdout if it was removed successfully.
        assert!(
            String::from_utf8(output.stdout)
                .expect("Could not decode daemon's response.")
                .contains(id),
            "{}",
            error_msg
        );
    }

    fn stop(&self, id: &str) {
        let _ = self
            .inner
            .command()
            .arg("stop")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command")
            .wait()
            .expect("Failed to stop docker container");
    }

    fn start(&self, id: &str) {
        self.inner
            .command()
            .arg("start")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command")
            .wait()
            .expect("Failed to start docker container");
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let networks = self.created_networks.read().expect("failed to lock RwLock");
        let created_networks = networks.len() > 0;

        match self.command {
            env::Command::Remove if created_networks => {
                self.delete_networks(networks.iter());
            }
            env::Command::Remove => {
                // nothing to do
            }
            env::Command::Keep => {
                let networks = networks.join(",");

                log::warn!(
                    "networks '{}' will not be automatically removed due to `TESTCONTAINERS` command",
                    networks
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::WaitFor, images::generic::GenericImage, Image, ImageExt};
    use spectral::prelude::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct HelloWorld {
        volumes: BTreeMap<String, String>,
        env_vars: BTreeMap<String, String>,
    }

    impl Image for HelloWorld {
        type Args = Vec<String>;

        fn descriptor(&self) -> String {
            String::from("hello-world")
        }

        fn ready_conditions(&self) -> Vec<WaitFor> {
            vec![WaitFor::message_on_stdout("Hello from Docker!")]
        }

        fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
            Box::new(self.env_vars.iter())
        }

        fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
            Box::new(self.volumes.iter())
        }
    }

    #[test]
    fn cli_run_command_should_include_env_vars() {
        let mut volumes = BTreeMap::new();
        volumes.insert("one-from".to_owned(), "one-dest".to_owned());
        volumes.insert("two-from".to_owned(), "two-dest".to_owned());

        let mut env_vars = BTreeMap::new();
        env_vars.insert("one-key".to_owned(), "one-value".to_owned());
        env_vars.insert("two-key".to_owned(), "two-value".to_owned());

        let image = HelloWorld { volumes, env_vars };

        let command = build_run_command(Command::new("docker"), &RunnableImage::from(image));

        assert_eq!(
            format!("{:?}", command),
            r#""docker" "run" "-e" "one-key=one-value" "-e" "two-key=two-value" "-v" "one-from:one-dest" "-v" "two-from:two-dest" "-P" "-d" "hello-world""#
        );
    }

    #[test]
    fn cli_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let command = build_run_command(
            Command::new("docker"),
            &RunnableImage::from(GenericImage::new("hello")),
        );

        assert_eq!(
            format!("{:?}", command),
            r#""docker" "run" "-P" "-d" "hello""#
        );
    }

    #[test]
    fn cli_run_command_should_expose_only_requested_ports() {
        let command = build_run_command(
            Command::new("docker"),
            &GenericImage::new("hello")
                .with_mapped_port((123, 456))
                .with_mapped_port((555, 888)),
        );

        assert_eq!(
            format!("{:?}", command),
            r#""docker" "run" "-p" "123:456" "-p" "555:888" "-d" "hello""#
        );
    }

    #[test]
    #[should_panic(expected = "Failed to remove docker container")]
    fn cli_rm_command_should_panic_on_invalid_container() {
        let docker = Cli::default();
        docker.rm("!INVALID_NAME_DUE_TO_SYMBOLS!");
        unreachable!()
    }

    #[test]
    fn cli_run_command_should_include_network() {
        let command = build_run_command(
            Command::new("docker"),
            &GenericImage::new("hello").with_network("awesome-net"),
        );
        assert_eq!(
            format!("{:?}", command),
            r#""docker" "run" "--network=awesome-net" "-P" "-d" "hello""#
        );
    }

    #[test]
    fn cli_run_command_should_include_name() {
        let command = build_run_command(
            Command::new("docker"),
            &GenericImage::new("hello").with_name("hello_container"),
        );

        assert_eq!(
            format!("{:?}", command),
            r#""docker" "run" "--name=hello_container" "-P" "-d" "hello""#
        );
    }

    #[test]
    fn should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        {
            let docker = Cli::default();

            assert!(!docker.inner.network_exists("awesome-net"));

            // creating the first container creates the network
            let _container1 = docker.run(HelloWorld::default().with_network("awesome-net"));
            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = docker.run(HelloWorld::default().with_network("awesome-net"));

            assert!(docker.inner.network_exists("awesome-net"));
        }

        {
            let docker = Cli::default();
            // original client has been dropped, should clean up networks
            assert!(!docker.inner.network_exists("awesome-net"))
        }
    }

    struct FakeEnvAlwaysKeep;

    impl GetEnvValue for FakeEnvAlwaysKeep {
        fn get_env_value(_: &str) -> Option<String> {
            Some("keep".to_owned())
        }
    }

    #[test]
    fn should_not_delete_network_if_command_is_keep() {
        let network_name = "foobar-net";

        {
            let docker = Cli::new::<FakeEnvAlwaysKeep, _>("docker");

            assert!(!docker.inner.network_exists(network_name));

            // creating the first container creates the network
            let _container1 = docker.run(HelloWorld::default().with_network(network_name));

            assert!(docker.inner.network_exists(network_name));
        }

        let docker = Cli::docker();

        assert!(
            docker.inner.network_exists(network_name),
            "network should still exist after client is dropped"
        );

        docker.inner.delete_networks(vec![network_name]);
    }

    #[test]
    fn should_wait_for_at_least_one_second_before_fetching_logs() {
        let _ = pretty_env_logger::try_init();
        let docker = Cli::default();

        let before_run = Instant::now();
        let container = docker.run(HelloWorld::default());
        let after_run = Instant::now();

        let before_logs = Instant::now();
        docker.stdout_logs(container.id());
        let after_logs = Instant::now();

        assert_that(&(after_run - before_run)).is_greater_than(Duration::from_secs(1));
        assert_that(&(after_logs - before_logs)).is_less_than(Duration::from_secs(1));
    }
}
