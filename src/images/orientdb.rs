use std::collections::HashMap;

use crate::{Container, Docker, Image, WaitForMessage};

const CONTAINER_IDENTIFIER: &str = "orientdb";
const DEFAULT_TAG: &str = "3.1.3";

#[derive(Debug, Default, Clone)]
pub struct OrientDBArgs;

impl IntoIterator for OrientDBArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct OrientDB {
    tag: String,
    arguments: OrientDBArgs,
    env_vars: HashMap<String, String>,
}

impl Default for OrientDB {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ORIENTDB_ROOT_PASSWORD".to_owned(), "root".to_owned());

        OrientDB {
            tag: DEFAULT_TAG.to_string(),
            arguments: OrientDBArgs {},
            env_vars,
        }
    }
}

impl Image for OrientDB {
    type Args = OrientDBArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        container
            .logs()
            .stderr
            .wait_for_message("OrientDB Studio available at")
            .unwrap();
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        OrientDB { arguments, ..self }
    }
}

impl OrientDB {
    pub fn with_tag(self, tag_str: &str) -> Self {
        OrientDB {
            tag: tag_str.to_string(),
            ..self
        }
    }

    pub fn with_env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}
