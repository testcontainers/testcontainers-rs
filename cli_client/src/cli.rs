use serde_json;
use std::collections::HashMap;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};
use tc_core::{self, Container, Docker, Image, Logs};

/// Implementation of the Docker client API using the docker cli.
///
/// This (fairly naive) implementation of the Docker client API simply creates `Command`s to the `docker` CLI. It thereby assumes that the `docker` CLI is installed and that it is in the PATH of the current execution environment.
#[derive(Debug, Default)]
pub struct Cli;

impl Docker for Cli {
    fn run<I: Image>(&self, image: I) -> Container<Cli, I> {
        let mut docker = Command::new("docker");

        let command = docker
            .arg("run")
            .arg("-d") // Always run detached
            .arg("-P") // Always expose all ports
            .arg(&image.descriptor())
            .args(image.args())
            .stdout(Stdio::piped());

        debug!("Executing command: {:?}", command);

        let child = command.spawn().expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();
        let reader = BufReader::new(stdout);

        let container_id = reader.lines().next().unwrap().unwrap();

        Container::new(container_id, self, image)
    }

    fn logs(&self, id: &str) -> Logs {
        // Hack to fix unstable CI builds. Sometimes the logs are not immediately available after starting the container.
        // Let's sleep for a little bit of time to let the container start up before we actually process the logs.
        sleep(Duration::from_millis(100));

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

    fn ports(&self, id: &str) -> tc_core::Ports {
        let child = Command::new("docker")
            .arg("inspect")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();

        let mut infos: Vec<ContainerInfo> = serde_json::from_reader(stdout).unwrap();

        let info = infos.remove(0);

        trace!("Fetched container info: {:#?}", info);

        info.network_settings.ports.into_ports()
    }

    fn rm(&self, id: &str) {
        Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg("-v") // Also remove volumes
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }

    fn stop(&self, id: &str) {
        Command::new("docker")
            .arg("stop")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute docker command");
    }
}

#[derive(Deserialize, Debug)]
struct NetworkSettings {
    #[serde(rename = "Ports")]
    ports: Ports,
}

#[derive(Deserialize, Debug)]
struct PortMapping {
    #[serde(rename = "HostIp")]
    ip: String,
    #[serde(rename = "HostPort")]
    port: String,
}

#[derive(Deserialize, Debug)]
struct ContainerInfo {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "NetworkSettings")]
    network_settings: NetworkSettings,
}

#[derive(Deserialize, Debug)]
struct Ports(HashMap<String, Option<Vec<PortMapping>>>);

impl Ports {
    pub fn into_ports(self) -> tc_core::Ports {
        let mut ports = tc_core::Ports::default();

        for (internal, external) in self.0 {
            let external = match external.and_then(|mut m| m.pop()).map(|m| m.port) {
                Some(port) => port,
                None => {
                    debug!("Port {} is not mapped to host machine, skipping.", internal);
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

    fn parse_port(port: &str) -> u32 {
        port.parse()
            .unwrap_or_else(|e| panic!("Failed to parse {} as u32 because {}", port, e))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json;

    #[test]
    fn can_deserialize_docker_inspect_response_into_api_ports() {
        let info =
            serde_json::from_str::<ContainerInfo>(include_str!("docker_inspect_response.json"))
                .unwrap();

        let parsed_ports = info.network_settings.ports.into_ports();
        let mut expected_ports = tc_core::Ports::default();

        expected_ports
            .add_mapping(18332, 33076)
            .add_mapping(18333, 33075)
            .add_mapping(8332, 33078)
            .add_mapping(8333, 33077);

        assert_eq!(parsed_ports, expected_ports)
    }

}
