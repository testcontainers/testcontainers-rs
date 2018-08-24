use std::{collections::HashMap, env::var, fmt, io::Read, str::FromStr};

pub trait Docker
where
    Self: Sized + Copy,
{
    fn new() -> Self;
    fn run<I: Image>(&self, image: I) -> Container<Self, I>;

    fn logs(&self, id: &str) -> Box<Read>;
    fn inspect(&self, id: &str) -> ContainerInfo;
    fn rm(&self, id: &str);
    fn stop(&self, id: &str);
}

pub trait Image
where
    Self: Sized + Default,
    Self::Args: IntoIterator<Item = String>,
{
    type Args;

    fn descriptor(&self) -> String;
    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>);
    fn args(&self) -> Self::Args;

    fn with_args(self, arguments: Self::Args) -> Self;
}

pub struct Container<D: Docker, I: Image> {
    id: String,
    docker_client: D,
    image: I,
}

impl<D: Docker, I: Image> fmt::Debug for Container<D, I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Container ({})", self.id)
    }
}

impl<D: Docker, I: Image> fmt::Display for Container<D, I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Container ({})", self.id)
    }
}

impl<D: Docker, I: Image> Container<D, I> {
    pub fn new(id: String, docker_client: D, image: I) -> Self {
        Container {
            id,
            docker_client,
            image,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn logs(&self) -> Box<Read> {
        self.docker_client.logs(&self.id)
    }

    pub fn get_host_port(&self, internal_port: u32) -> Option<u32> {
        let resolved_port = self
            .docker_client
            .inspect(&self.id)
            .network_settings()
            .ports()
            .map_to_external_port(internal_port);

        match resolved_port {
            Some(port) => {
                debug!(
                    "Resolved port {} to {} for container {}",
                    internal_port, port, self.id
                );
            }
            None => {
                warn!(
                    "Unable to resolve port {} for container {}",
                    internal_port, self.id
                );
            }
        }

        resolved_port
    }

    pub(crate) fn block_until_ready(&self) {
        self.image.wait_until_ready(self);
    }

    pub fn image(&self) -> &I {
        &self.image
    }

    pub fn stop(&self) {
        self.docker_client.stop(&self.id)
    }

    pub fn rm(&self) {
        self.docker_client.rm(&self.id)
    }
}

impl<D: Docker, I: Image> Drop for Container<D, I> {
    fn drop(&mut self) {
        let keep_container = var("KEEP_CONTAINERS")
            .ok()
            .and_then(|var| var.parse().ok())
            .unwrap_or(false);

        match keep_container {
            true => self.stop(),
            false => self.rm(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ContainerInfo {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "NetworkSettings")]
    network_settings: NetworkSettings,
}

impl ContainerInfo {
    pub fn network_settings(&self) -> &NetworkSettings {
        &self.network_settings
    }
}

#[derive(Deserialize, Debug)]
pub struct NetworkSettings {
    #[serde(rename = "Ports")]
    ports: Ports,
}

impl NetworkSettings {
    pub fn ports(&self) -> &Ports {
        &self.ports
    }
}

#[derive(Deserialize, Debug)]
pub struct Ports(HashMap<String, Option<Vec<PortMapping>>>);

#[derive(Deserialize, Debug)]
pub struct PortMapping {
    #[serde(rename = "HostIp")]
    ip: String,
    #[serde(rename = "HostPort")]
    port: String,
}

impl Ports {
    pub fn map_to_external_port(&self, internal_port: u32) -> Option<u32> {
        for key in self.0.keys() {
            let internal_port = format!("{}", internal_port);
            if key.contains(&internal_port) {
                return self.0.get(key).and_then(|option| {
                    option
                        .as_ref()
                        .and_then(|mappings| mappings.get(0))
                        .map(|mapping| &mapping.port)
                        .and_then(|port| u32::from_str(port).ok())
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    extern crate serde_json;

    #[test]
    fn can_deserialize_docker_inspect_response() {
        let response = r#"{
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
    }"#;

        let info = serde_json::from_str::<ContainerInfo>(response).unwrap();

        let ports = info.network_settings.ports;

        let external_port = ports.map_to_external_port(18332);

        assert_eq!(
            info.id,
            "fd2e896b883052dae31202b065a06dc5374a214ae348b7a8f8da3734f690d010"
        );
        assert_eq!(external_port, Some(33076));
    }

}
