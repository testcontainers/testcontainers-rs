use crate::core::Port;

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
