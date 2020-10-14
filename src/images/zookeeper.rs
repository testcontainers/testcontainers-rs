use crate::core::Port;
use crate::{Container, Docker, Image, WaitForMessage};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "zookeeper";
const DEFAULT_TAG: &str = "3.6.2";

#[derive(Debug, Default, Clone)]
pub struct ZookeeperArgs;
impl IntoIterator for ZookeeperArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct Zookeeper {
    tag: String,
    arguments: ZookeeperArgs,
    ports: Option<Vec<Port>>,
}

impl Default for Zookeeper {
    fn default() -> Self {
        Zookeeper {
            tag: DEFAULT_TAG.to_string(),
            arguments: ZookeeperArgs {},
            ports: None,
        }
    }
}
impl Image for Zookeeper {
    type Args = ZookeeperArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;
    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Started AdminServer")
            .unwrap();
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn ports(&self) -> Option<Vec<Port>> {
        self.ports.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        Zookeeper { arguments, ..self }
    }
}
impl Zookeeper {
    pub fn with_tag(self, tag_str: &str) -> Self {
        Zookeeper {
            tag: tag_str.to_string(),
            ..self
        }
    }

    pub fn with_mapped_port<P: Into<Port>>(mut self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());
        self.ports = Some(ports);
        self
    }
}
