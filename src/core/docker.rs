use crate::core::{logs::LogStream, ports::Ports, PortMapping};

/// Container run command arguments.
/// `name` - run image instance with the given name (should be explicitly set to be seen by other containers created in the same docker network).
/// `network` - run image instance on the given network.
/// `ports` - run image instance with the given ports mapping (if explicit mappings is not defined, all image ports will be automatically exposed and mapped on random host ports).
#[derive(Debug, Clone, Default)]
pub struct RunArgs {
    name: Option<String>,
    network: Option<String>,
    ports: Option<Vec<PortMapping>>,
}

/// Defines operations that we need to perform on docker containers and other entities.
///
/// This trait is pub(crate) because it should not be used directly by users but only represents an internal abstraction that allows containers to be generic over the client they have been started with.
/// All functionality of this trait is available on [`Container`]s directly.
pub(crate) trait Docker {
    fn stdout_logs(&self, id: &str) -> LogStream;
    fn stderr_logs(&self, id: &str) -> LogStream;
    fn ports(&self, id: &str) -> Ports;
    fn rm(&self, id: &str);
    fn stop(&self, id: &str);
    fn start(&self, id: &str);
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

    pub fn with_mapped_port<P: Into<PortMapping>>(mut self, port: P) -> Self {
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

    pub(crate) fn ports(&self) -> Option<Vec<PortMapping>> {
        self.ports.clone()
    }
}
