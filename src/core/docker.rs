use crate::core::Port;
use crate::{Container, Image};
use std::{collections::HashMap, fmt, io::Read};

/// Defines the minimum API required for interacting with the Docker daemon.
pub trait Docker
where
    Self: Sized,
{
    fn run<I: Image>(&self, image: I) -> Container<'_, Self, I>;
    fn run_with_args<I: Image>(&self, image: I, run_args: RunArgs) -> Container<'_, Self, I>;
    fn logs(&self, id: &str) -> Logs;
    fn ports(&self, id: &str) -> Ports;
    fn rm(&self, id: &str);
    fn stop(&self, id: &str);
    fn start(&self, id: &str);
}

/// Container run command arguments.
/// `name` - run image instance with the given name (should be explicitly set to be seen by other containers created in the same docker network).
/// `network` - run image instance on the given network.
/// `ports` - run image instance with the given ports mapping (if explicit mappings is not defined, all image ports will be automatically exposed and mapped on random host ports).
#[derive(Debug, Clone, Default)]
pub struct RunArgs {
    name: Option<String>,
    network: Option<String>,
    ports: Option<Vec<Port>>,
}

impl RunArgs {
    pub fn with_name<T: ToString>(self, name: T) -> Self {
        RunArgs {
            name: Some(name.to_string()),
            ..self
        }
    }

    pub fn with_network<T: ToString>(self, network: T) -> Self {
        RunArgs {
            network: Some(network.to_string()),
            ..self
        }
    }

    pub fn with_mapped_port<P: Into<Port>>(mut self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());
        self.ports = Some(ports);
        self
    }

    pub(crate) fn network(&self) -> Option<String> {
        self.network.clone()
    }

    pub(crate) fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub(crate) fn ports(&self) -> Option<Vec<Port>> {
        self.ports.clone()
    }
}

/// The exposed ports of a running container.
#[derive(Debug, PartialEq, Default)]
pub struct Ports {
    mapping: HashMap<u16, u16>,
}

impl Ports {
    /// Registers the mapping of an exposed port.
    pub fn add_mapping(&mut self, internal: u16, host: u16) -> &mut Self {
        log::debug!("Registering port mapping: {} -> {}", internal, host);

        self.mapping.insert(internal, host);

        self
    }

    /// Returns the host port for the given internal port.
    pub fn map_to_host_port(&self, internal_port: u16) -> Option<u16> {
        self.mapping.get(&internal_port).cloned()
    }
}

/// Log streams of running container (stdout & stderr).
pub struct Logs {
    pub stdout: Box<dyn Read>,
    pub stderr: Box<dyn Read>,
}

impl fmt::Debug for Logs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Logs").finish()
    }
}
