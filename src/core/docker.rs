use crate::{Container, Image};
use std::{collections::HashMap, io::Read};

/// Defines the minimum API required for interacting with the Docker daemon.
pub trait Docker
where
    Self: Sized,
{
    fn run<I: Image>(&self, image: I) -> Container<'_, Self, I>;
    fn logs(&self, id: &str) -> Logs;
    fn ports(&self, id: &str) -> Ports;
    fn rm(&self, id: &str);
    fn stop(&self, id: &str);
}

/// The exposed ports of a running container.
#[derive(Debug, PartialEq, Default)]
pub struct Ports {
    mapping: HashMap<u32, u32>,
}

impl Ports {
    /// Registers the mapping of an exposed port.
    pub fn add_mapping(&mut self, internal: u32, host: u32) -> &mut Self {
        log::debug!("Registering port mapping: {} -> {}", internal, host);

        self.mapping.insert(internal, host);

        self
    }

    /// Returns the host port for the given internal port.
    pub fn map_to_host_port(&self, internal_port: u32) -> Option<u32> {
        self.mapping.get(&internal_port).map(|p| p.clone())
    }
}

/// Log streams of running container (stdout & stderr).
#[derive(derivative::Derivative)]
#[derivative(Debug)]
pub struct Logs {
    #[derivative(Debug = "ignore")]
    pub stdout: Box<dyn Read>,
    #[derivative(Debug = "ignore")]
    pub stderr: Box<dyn Read>,
}
