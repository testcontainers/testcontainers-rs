use crate::core::Port;
use crate::{Container, Docker, Image, WaitForMessage};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "bitnami/kafka";
const DEFAULT_TAG: &str = "2.6.0";

#[derive(Debug, Default, Clone)]
pub struct KafkaArgs;

impl IntoIterator for KafkaArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct Kafka {
    tag: String,
    arguments: KafkaArgs,
    ports: Option<Vec<Port>>,
    env_vars: HashMap<String, String>,
}

impl Default for Kafka {
    fn default() -> Self {
        Kafka {
            tag: DEFAULT_TAG.to_string(),
            arguments: KafkaArgs {},
            ports: None,
            env_vars: HashMap::default(),
        }
    }
}

impl Image for Kafka {
    type Args = KafkaArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;
    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        container.logs().stdout.wait_for_message("started").unwrap();
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        Kafka { arguments, ..self }
    }
}

impl Kafka {
    pub fn ports(&self) -> Option<Vec<Port>> {
        self.ports.clone()
    }

    pub fn with_tag(self, tag_str: &str) -> Self {
        Kafka {
            tag: tag_str.to_string(),
            ..self
        }
    }

    pub fn with_env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}
