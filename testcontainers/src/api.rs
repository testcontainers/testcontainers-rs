use std::{collections::HashMap, env::var, fmt, io::Read};

pub trait Docker
where
    Self: Sized,
{
    fn new() -> Self;
    fn run<I: Image>(&self, image: I) -> Container<Self, I>;

    fn logs(&self, id: &str) -> Logs;
    fn ports(&self, id: &str) -> Ports;
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

impl<'d, D, I> fmt::Debug for Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Container ({})", self.id)
    }
}

impl<'d, D, I> fmt::Display for Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Container ({})", self.id)
    }
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

#[derive(Debug, PartialEq)]
pub struct Ports {
    pub(crate) mapping: HashMap<u32, u32>,
}

impl Ports {
    pub fn map_to_external_port(&self, port: u32) -> Option<u32> {
        self.mapping.get(&port).map(|p| p.clone())
    }
}

pub struct Logs {
    pub stdout: Box<Read>,
    pub stderr: Box<Read>,
}
