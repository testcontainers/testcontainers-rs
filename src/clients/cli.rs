use crate::core::{self, Container, Docker, Image, Logs, RunArgs};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use std::{
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

const ONE_SECOND: Duration = Duration::from_secs(1);
const ZERO: Duration = Duration::from_secs(0);

/// Implementation of the Docker client API using the docker cli.
///
/// This (fairly naive) implementation of the Docker client API simply creates `Command`s to the `docker` CLI. It thereby assumes that the `docker` CLI is installed and that it is in the PATH of the current execution environment.
#[derive(Debug, Default)]
pub struct Cli {
    /// The docker CLI has an issue that if you request logs for a container
    /// too quickly after it was started up, the resulting stream will never
    /// emit any data, even if the container is already emitting logs.
    ///
    /// We keep track of when we started a container in order to make sure
    /// that we wait at least one second after that. Subsequent invocations
    /// directly fetch the logs of a container.
    container_startup_timestamps: RwLock<HashMap<String, Instant>>,
    created_networks: RwLock<Vec<String>>,
}

impl Cli {
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
                sleep(ONE_SECOND.checked_sub(duration).unwrap_or_else(|| ZERO))
            }
        }
    }

    fn build_run_command<'a, I: Image>(
        image: &I,
        command: &'a mut Command,
        run_args: &RunArgs,
    ) -> &'a mut Command {
        command.arg("run");

        if let Some(network) = run_args.network() {
            command.arg(format!("--network={}", network));
        }

        if let Some(name) = run_args.name() {
            command.arg(format!("--name={}", name));
        }

        for (key, value) in image.env_vars() {
            command.arg("-e").arg(format!("{}={}", key, value));
        }

        for (orig, dest) in image.volumes() {
            command.arg("-v").arg(format!("{}:{}", orig, dest));
        }

        if let Some(entrypoint) = image.entrypoint() {
            command.arg("--entrypoint").arg(entrypoint);
        }

        if let Some(ports) = run_args.ports() {
            for port in &ports {
                command
                    .arg("-p")
                    .arg(format!("{}:{}", port.local, port.internal));
            }
        } else {
            command.arg("-P"); // expose all ports
        }

        command
            .arg("-d") // Always run detached
            .arg(image.descriptor())
            .args(image.args())
            .stdout(Stdio::piped())
    }
}

impl Docker for Cli {
    fn run<I: Image>(&self, image: I) -> Container<'_, Cli, I> {
        let empty_args = RunArgs::default();
        self.run_with_args(image, empty_args)
    }

    fn run_with_args<I: Image>(&self, image: I, run_args: RunArgs) -> Container<'_, Cli, I> {
        let mut docker = Command::new("docker");

        if let Some(network) = run_args.network() {
            if create_network_if_not_exists(&network) {
                let mut guard = self
                    .created_networks
                    .write()
                    .expect("failed to lock RwLock");

                guard.push(network);
            }
        }

        let command = Cli::build_run_command(&image, &mut docker, &run_args);

        log::debug!("Executing command: {:?}", command);

        let output = command.output().expect("Failed to execute docker command");

        assert!(output.status.success(), "failed to start container");
        let container_id = String::from_utf8(output.stdout)
            .expect("output is not valid utf8")
            .trim()
            .to_string();
        self.register_container_started(container_id.clone());

        Container::new(container_id, self, image)
    }

    fn logs(&self, id: &str) -> Logs {
        self.wait_at_least_one_second_after_container_was_started(id);

        let child = Command::new("docker")
            .arg("logs")
            .arg("-f")
            .arg(id)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        Logs {
            stdout: Box::new(child.stdout.unwrap()),
            stderr: Box::new(child.stderr.unwrap()),
        }
    }

    fn ports(&self, id: &str) -> crate::core::Ports {
        let child = Command::new("docker")
            .arg("inspect")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();

        let mut infos: Vec<ContainerInfo> = serde_json::from_reader(stdout).unwrap();

        let info = infos.remove(0);

        log::trace!("Fetched container info: {:#?}", info);

        info.network_settings.ports.into_ports()
    }

    fn rm(&self, id: &str) {
        let output = Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg("-v") // Also remove volumes
            .arg(id)
            .output()
            .expect("Failed to execute docker command");
        assert!(output.status.success(), "Failed to remove docker container");
    }

    fn stop(&self, id: &str) {
        let _ = Command::new("docker")
            .arg("stop")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command")
            .wait()
            .expect("Failed to stop docker container");
    }

    fn start(&self, id: &str) {
        Command::new("docker")
            .arg("start")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command")
            .wait()
            .expect("Failed to start docker container");
    }
}

impl Drop for Cli {
    fn drop(&mut self) {
        let guard = self.created_networks.read().expect("failed to lock RwLock");

        if guard.len() > 0 {
            let mut docker = Command::new("docker");
            docker.args(&["network", "rm"]);

            for network in guard.iter() {
                docker.arg(network);
            }

            let output = docker.output().expect("failed to delete docker networks");

            assert!(
                output.status.success(),
                "failed to delete docker networks: {}",
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }
}

fn create_network_if_not_exists(name: &str) -> bool {
    if network_exists(name) {
        return false;
    }

    let mut docker = Command::new("docker");
    docker.args(&["network", "create", name]);

    let output = docker.output().expect("failed to create docker network");
    assert!(output.status.success(), "failed to create docker network");

    true
}

fn network_exists(name: &str) -> bool {
    let mut docker = Command::new("docker");
    docker.args(&["network", "ls", "--format", "{{.Name}}"]);

    let output = docker.output().expect("failed to list docker networks");
    let output = String::from_utf8(output.stdout).expect("output is not valid utf-8");

    output.lines().any(|network| network == name)
}

#[derive(serde::Deserialize, Debug)]
struct NetworkSettings {
    #[serde(rename = "Ports")]
    ports: Ports,
}

#[derive(serde::Deserialize, Debug)]
struct PortMapping {
    #[serde(rename = "HostIp")]
    ip: String,
    #[serde(rename = "HostPort")]
    port: String,
}

#[derive(serde::Deserialize, Debug)]
struct ContainerInfo {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "NetworkSettings")]
    network_settings: NetworkSettings,
}

#[derive(serde::Deserialize, Debug)]
struct Ports(HashMap<String, Option<Vec<PortMapping>>>);

impl Ports {
    pub fn into_ports(self) -> core::Ports {
        let mut ports = core::Ports::default();

        for (internal, external) in self.0 {
            let external = match external.and_then(|mut m| m.pop()).map(|m| m.port) {
                Some(port) => port,
                None => {
                    log::debug!("Port {} is not mapped to host machine, skipping.", internal);
                    continue;
                }
            };

            let port = internal.split('/').next().unwrap();

            let internal = Self::parse_port(port);
            let external = Self::parse_port(&external);

            ports.add_mapping(internal, external);
        }

        ports
    }

    fn parse_port(port: &str) -> u16 {
        port.parse()
            .unwrap_or_else(|e| panic!("Failed to parse {} as u16 because {}", port, e))
    }
}

#[cfg(test)]
mod tests {
    use crate::images::generic::GenericImage;
    use crate::{Container, Docker, Image};

    use super::*;

    #[test]
    fn can_deserialize_docker_inspect_response_into_api_ports() {
        let info = serde_json::from_str::<ContainerInfo>(
            r#"{
  "Id": "fd2e896b883052dae31202b065a06dc5374a214ae348b7a8f8da3734f690d010",
  "NetworkSettings": {
    "Ports": {
      "18332/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33076"
        }
      ],
      "18333/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33075"
        }
      ],
      "18443/tcp": null,
      "18444/tcp": null,
      "8332/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33078"
        }
      ],
      "8333/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33077"
        }
      ]
    }
  }
}"#,
        )
        .unwrap();

        let parsed_ports = info.network_settings.ports.into_ports();
        let mut expected_ports = core::Ports::default();

        expected_ports
            .add_mapping(18332, 33076)
            .add_mapping(18333, 33075)
            .add_mapping(8332, 33078)
            .add_mapping(8333, 33077);

        assert_eq!(parsed_ports, expected_ports)
    }

    #[derive(Default)]
    struct HelloWorld {
        volumes: HashMap<String, String>,
        env_vars: HashMap<String, String>,
    }

    impl Image for HelloWorld {
        type Args = Vec<String>;
        type EnvVars = HashMap<String, String>;
        type Volumes = HashMap<String, String>;
        type EntryPoint = std::convert::Infallible;

        fn descriptor(&self) -> String {
            String::from("hello-world")
        }

        fn wait_until_ready<D: Docker>(&self, _container: &Container<'_, D, Self>) {}

        fn args(&self) -> <Self as Image>::Args {
            vec![]
        }

        fn volumes(&self) -> Self::Volumes {
            self.volumes.clone()
        }

        fn env_vars(&self) -> Self::EnvVars {
            self.env_vars.clone()
        }

        fn with_args(self, _arguments: <Self as Image>::Args) -> Self {
            self
        }
    }

    #[test]
    fn cli_run_command_should_include_env_vars() {
        let mut volumes = HashMap::new();
        volumes.insert("one-from".to_owned(), "one-dest".to_owned());
        volumes.insert("two-from".to_owned(), "two-dest".to_owned());

        let mut env_vars = HashMap::new();
        env_vars.insert("one-key".to_owned(), "one-value".to_owned());
        env_vars.insert("two-key".to_owned(), "two-value".to_owned());

        let image = HelloWorld { volumes, env_vars };

        let mut docker = Command::new("docker");
        let run_args = RunArgs::default();
        let command = Cli::build_run_command(&image, &mut docker, &run_args);

        println!("Executing command: {:?}", command);

        assert!(format!("{:?}", command).contains(r#"-d"#));
        assert!(format!("{:?}", command).contains(r#"-P"#));
        assert!(format!("{:?}", command).contains(r#""-v" "one-from:one-dest"#));
        assert!(format!("{:?}", command).contains(r#""-v" "two-from:two-dest"#));
        assert!(format!("{:?}", command).contains(r#""-e" "one-key=one-value""#));
        assert!(format!("{:?}", command).contains(r#""-e" "two-key=two-value""#));
    }

    #[test]
    fn cli_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let image = GenericImage::new("hello");

        let mut docker = Command::new("docker");
        let run_args = RunArgs::default();
        let command = Cli::build_run_command(&image, &mut docker, &run_args);

        println!("Executing command: {:?}", command);

        assert!(format!("{:?}", command).contains(r#"-d"#));
        assert!(format!("{:?}", command).contains(r#"-P"#));
        assert!(!format!("{:?}", command).contains(r#"-p"#));
    }

    #[test]
    fn cli_run_command_should_expose_only_requested_ports() {
        let image = GenericImage::new("hello");
        let mut docker = Command::new("docker");
        let run_args = RunArgs::default()
            .with_mapped_port((123, 456))
            .with_mapped_port((555, 888));
        let command = Cli::build_run_command(&image, &mut docker, &run_args);

        println!("Executing command: {:?}", command);

        assert!(format!("{:?}", command).contains(r#"-d"#));
        assert!(!format!("{:?}", command).contains(r#"-P"#));
        assert!(format!("{:?}", command).contains(r#""-p" "123:456""#));
        assert!(format!("{:?}", command).contains(r#""-p" "555:888""#));
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
        let image = GenericImage::new("hello");

        let mut docker = Command::new("docker");
        let run_args = RunArgs::default().with_network("awesome-net");
        let command = Cli::build_run_command(&image, &mut docker, &run_args);

        println!("Executing command: {:?}", command);

        assert!(format!("{:?}", command).contains(r#"--network=awesome-net"#));
    }

    #[test]
    fn cli_run_command_should_include_name() {
        let image = GenericImage::new("hello");

        let mut docker = Command::new("docker");
        let run_args = RunArgs::default().with_name("hello_container");
        let command = Cli::build_run_command(&image, &mut docker, &run_args);

        println!("Executing command: {:?}", command);

        assert!(format!("{:?}", command).contains(r#"--name=hello_container"#));
    }

    #[test]
    fn should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        {
            let docker = Cli::default();

            assert!(!network_exists("awesome-net"));

            // creating the first container creates the network
            let _container1 = docker.run_with_args(
                HelloWorld::default(),
                RunArgs::default().with_network("awesome-net"),
            );
            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = docker.run_with_args(
                HelloWorld::default(),
                RunArgs::default().with_network("awesome-net"),
            );

            assert!(network_exists("awesome-net"));
        }

        // client has been dropped, should clean up networks
        assert!(!network_exists("awesome-net"))
    }
}
