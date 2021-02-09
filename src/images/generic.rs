use crate::{core::WaitFor, Image};
use std::collections::HashMap;

#[derive(Debug)]
pub struct GenericImage {
    descriptor: String,
    arguments: Vec<String>,
    volumes: HashMap<String, String>,
    env_vars: HashMap<String, String>,
    wait_for: Vec<WaitFor>,
    entrypoint: Option<String>,
}

impl Default for GenericImage {
    fn default() -> Self {
        Self {
            descriptor: "".to_owned(),
            arguments: vec![],
            volumes: HashMap::new(),
            env_vars: HashMap::new(),
            wait_for: Vec::new(),
            entrypoint: None,
        }
    }
}

impl GenericImage {
    pub fn new<S: Into<String>>(descriptor: S) -> GenericImage {
        Self {
            descriptor: descriptor.into(),
            ..Default::default()
        }
    }

    pub fn with_volume<F: Into<String>, D: Into<String>>(mut self, from: F, dest: D) -> Self {
        self.volumes.insert(from.into(), dest.into());
        self
    }

    pub fn with_env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    pub fn with_wait_for(mut self, wait_for: WaitFor) -> Self {
        self.wait_for.push(wait_for);
        self
    }

    pub fn with_entrypoint(mut self, entrypoint: &str) -> Self {
        self.entrypoint = Some(entrypoint.to_string());
        self
    }
}

impl Image for GenericImage {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = str;

    fn descriptor(&self) -> String {
        self.descriptor.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        self.wait_for.clone()
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        self.volumes.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }

    fn entrypoint(&self) -> Option<String> {
        self.entrypoint.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_env_vars() {
        let image = GenericImage::new("hello")
            .with_env_var("one-key", "one-value")
            .with_env_var("two-key", "two-value");

        let env_vars = image.env_vars();
        assert_eq!(2, env_vars.len());
        assert_eq!("one-value", env_vars.get("one-key").unwrap());
        assert_eq!("two-value", env_vars.get("two-key").unwrap());
    }
}
