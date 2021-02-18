use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "orientdb";
const DEFAULT_TAG: &str = "3.1.3";

#[derive(Debug, Default, Clone)]
pub struct OrientDbArgs;

impl IntoIterator for OrientDbArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct OrientDb {
    tag: String,
    arguments: OrientDbArgs,
    env_vars: HashMap<String, String>,
}

impl Default for OrientDb {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ORIENTDB_ROOT_PASSWORD".to_owned(), "root".to_owned());

        OrientDb {
            tag: DEFAULT_TAG.to_string(),
            arguments: OrientDbArgs {},
            env_vars,
        }
    }
}

impl Image for OrientDb {
    type Args = OrientDbArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("OrientDB Studio available at")]
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
        OrientDb { arguments, ..self }
    }
}

impl OrientDb {
    pub fn with_tag(self, tag_str: &str) -> Self {
        OrientDb {
            tag: tag_str.to_string(),
            ..self
        }
    }

    pub fn with_env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}
