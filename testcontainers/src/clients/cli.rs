use api;
use serde_json;
use std::collections::HashMap;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

pub struct Cli;

impl api::Docker for Cli {
    fn new() -> Self {
        Cli
    }

    fn run<I: api::Image>(&self, image: I) -> api::Container<Cli, I> {
        let mut docker = Command::new("docker");

        let command = docker
            .arg("run")
            .arg("-d") // Always run detached
            .arg("-P") // Always expose all ports
            .arg(&image.descriptor())
            .args(image.args())
            .stdout(Stdio::piped());

        info!("Executing command: {:?}", command);

        let child = command.spawn().expect("Failed to execute docker command");

        let stdout = child.stdout.unwrap();
        let reader = BufReader::new(stdout);

        let container_id = reader.lines().next().unwrap().unwrap();

        // TODO maybe move log statements to container
        let container = api::Container::new(container_id, self, image);

        debug!("Waiting for {} to be ready.", container);

        container.block_until_ready();

        debug!("{} is now ready!", container);

        container
    }

    fn logs(&self, id: &str) -> api::Logs {
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

        api::Logs {
            stdout: Box::new(child.stdout.unwrap()),
            stderr: Box::new(child.stderr.unwrap()),
        }
    }

    fn ports(&self, id: &str) -> api::Ports {
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
        info!("Killing docker container: {}", id);

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
        info!("Stopping docker container: {}", id);

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
    pub fn into_ports(self) -> ::api::Ports {
        let mut mapping = HashMap::new();

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

            mapping.insert(internal, external);
        }

        api::Ports { mapping }
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

        let ports = info.network_settings.ports.into_ports();

        assert_eq!(
            ports,
            ::api::Ports {
                mapping: hashmap!{
                    18332 => 33076,
                    18333 => 33075,
                    8332 => 33078,
                    8333 => 33077,
                },
            }
        )
    }

}
