use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "eventstore/eventstore";
const DEFAULT_TAG: &str = "20.10.0-bionic";

#[derive(Debug, Default, Clone)]
pub struct EventStoreDBArgs;

impl IntoIterator for EventStoreDBArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct EventStoreDB {
    tag: String,
    arguments: EventStoreDBArgs,
    env_vars: HashMap<String, String>,
}

impl EventStoreDB {
    pub fn insecure_mode(mut self) -> Self {
        self.env_vars
            .insert("EVENTSTORE_INSECURE".to_string(), "true".to_string());
        self.env_vars.insert(
            "EVENTSTORE_ENABLE_ATOM_PUB_OVER_HTTP".to_string(),
            "true".to_string(),
        );

        self
    }

    pub fn enable_projections(mut self) -> Self {
        self.env_vars
            .insert("EVENTSTORE_RUN_PROJECTIONS".to_string(), "all".to_string());
        self.env_vars.insert(
            "EVENTSTORE_START_STANDARD_PROJECTIONS".to_string(),
            "true".to_string(),
        );

        self
    }

    pub fn with_tag(self, tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            ..self
        }
    }

    pub fn add_env_var(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());

        self
    }
}

impl Image for EventStoreDB {
    type Args = EventStoreDBArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("SPARTA")]
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        EventStoreDB { arguments, ..self }
    }
}

impl Default for EventStoreDB {
    fn default() -> Self {
        EventStoreDB {
            tag: DEFAULT_TAG.to_string(),
            arguments: EventStoreDBArgs::default(),
            env_vars: HashMap::new(),
        }
    }
}
