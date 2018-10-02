use std::{collections::HashMap, env::var, io::Read};
use Docker;
use Image;

#[derive(Debug)]
pub struct Container<'d, D, I>
where
    D: 'd,
    D: Docker,
    I: Image,
{
    id: String,
    docker_client: &'d D,
    image: I,
}

impl<'d, D, I> Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
    pub fn new(id: String, docker_client: &'d D, image: I) -> Self {
        Container {
            id,
            docker_client,
            image,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn logs(&self) -> Logs {
        self.docker_client.logs(&self.id)
    }

    pub fn get_host_port(&self, internal_port: u32) -> Option<u32> {
        let resolved_port = self
            .docker_client
            .ports(&self.id)
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

    pub fn block_until_ready(&self) {
        debug!("Waiting for container {} to be ready", self.id);

        self.image.wait_until_ready(self);

        debug!("Container {} is now ready!", self.id);
    }

    pub fn image(&self) -> &I {
        &self.image
    }

    pub fn stop(&self) {
        debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id)
    }

    pub fn rm(&self) {
        debug!("Deleting docker container {}", self.id);

        self.docker_client.rm(&self.id)
    }
}

impl<'d, D, I> Drop for Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
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

#[derive(Debug, PartialEq, Default)]
pub struct Ports {
    mapping: HashMap<u32, u32>,
}

impl Ports {
    pub fn add_mapping(&mut self, internal: u32, external: u32) -> &mut Self {
        debug!("Registering port mapping: {} -> {}", internal, external);

        self.mapping.insert(internal, external);

        self
    }

    pub fn map_to_external_port(&self, port: u32) -> Option<u32> {
        self.mapping.get(&port).map(|p| p.clone())
    }
}

#[derive(DebugStub)]
pub struct Logs {
    #[debug_stub = "stream"]
    pub stdout: Box<Read>,
    #[debug_stub = "stream"]
    pub stderr: Box<Read>,
}
