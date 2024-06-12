use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    core::{mounts::Mount, WaitFor},
    Image,
};
use crate::core::ports::{ExposedPort};

#[must_use]
#[derive(Debug, Clone)]
pub struct GenericImage {
    name: String,
    tag: String,
    mounts: Vec<Mount>,
    env_vars: BTreeMap<String, String>,
    wait_for: Vec<WaitFor>,
    entrypoint: Option<String>,
    cmd: Vec<String>,
    exposed_ports: Vec<ExposedPort>,
}

impl Default for GenericImage {
    fn default() -> Self {
        Self {
            name: "".to_owned(),
            tag: "".to_owned(),
            mounts: Vec::new(),
            env_vars: BTreeMap::new(),
            cmd: Vec::new(),
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

    pub fn with_mount(mut self, mount: impl Into<Mount>) -> Self {
        self.mounts.push(mount.into());
        self
    }

    pub fn with_env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    pub fn with_cmd(mut self, cmd: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.cmd = cmd.into_iter().map(Into::into).collect();
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

    pub fn with_exposed_port<P: Into<ExposedPort>>(mut self, exposed_port: P) -> Self {
        self.exposed_ports.push(exposed_port.into());
        self
    }
}

impl Image for GenericImage {
    fn name(&self) -> &str {
        &self.name
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        self.wait_for.clone()
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        &self.cmd
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        &self.mounts
    }

    fn entrypoint(&self) -> Option<&str> {
        self.entrypoint.as_deref()
    }

    fn expose_ports(&self) -> &[ExposedPort] {
        &self.exposed_ports
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

        let mut env_vars = image.env_vars().into_iter();
        let (first_key, first_value) = env_vars.next().unwrap();
        let (second_key, second_value) = env_vars.next().unwrap();

        assert_eq!(first_key.into(), "one-key");
        assert_eq!(first_value.into(), "one-value");
        assert_eq!(second_key.into(), "two-key");
        assert_eq!(second_value.into(), "two-value");
    }
}
