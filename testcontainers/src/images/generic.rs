use crate::{core::WaitFor, Image, ImageArgs};
use std::collections::BTreeMap;

impl ImageArgs for Vec<String> {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(self.into_iter())
    }
}

#[must_use]
#[derive(Debug, Clone)]
pub struct GenericImage {
    name: String,
    tag: String,
    volumes: BTreeMap<String, String>,
    env_vars: BTreeMap<String, String>,
    wait_for: Vec<WaitFor>,
    entrypoint: Option<String>,
    exposed_ports: Vec<u16>,
}

impl Default for GenericImage {
    fn default() -> Self {
        Self {
            name: "".to_owned(),
            tag: "".to_owned(),
            volumes: BTreeMap::new(),
            env_vars: BTreeMap::new(),
            wait_for: Vec::new(),
            entrypoint: None,
            exposed_ports: Vec::new(),
        }
    }
}

impl GenericImage {
    pub fn new<S: Into<String>>(name: S, tag: S) -> GenericImage {
        Self {
            name: name.into(),
            tag: tag.into(),
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

    pub fn with_exposed_port(mut self, port: u16) -> Self {
        self.exposed_ports.push(port);
        self
    }
}

impl Image for GenericImage {
    type Args = Vec<String>;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        self.wait_for.clone()
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.volumes.iter())
    }

    fn entrypoint(&self) -> Option<String> {
        self.entrypoint.clone()
    }

    fn expose_ports(&self) -> Vec<u16> {
        self.exposed_ports.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_env_vars() {
        let image = GenericImage::new("hello-world", "latest")
            .with_env_var("one-key", "one-value")
            .with_env_var("two-key", "two-value");

        let mut env_vars = image.env_vars();
        let (first_key, first_value) = env_vars.next().unwrap();
        let (second_key, second_value) = env_vars.next().unwrap();

        assert_eq!(first_key, "one-key");
        assert_eq!(first_value, "one-value");
        assert_eq!(second_key, "two-key");
        assert_eq!(second_value, "two-value");
    }
}
