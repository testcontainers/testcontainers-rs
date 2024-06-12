use crate::{core::WaitFor, Image};

#[must_use]
#[derive(Debug, Clone)]
pub struct GenericImage {
    name: String,
    tag: String,
    wait_for: Vec<WaitFor>,
    entrypoint: Option<String>,
    exposed_ports: Vec<u16>,
}

impl GenericImage {
    pub fn new<S: Into<String>>(name: S, tag: S) -> GenericImage {
        Self {
            name: name.into(),
            tag: tag.into(),
            wait_for: Vec::new(),
            entrypoint: None,
            exposed_ports: Vec::new(),
        }
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
    fn name(&self) -> &str {
        &self.name
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        self.wait_for.clone()
    }

    fn entrypoint(&self) -> Option<&str> {
        self.entrypoint.as_deref()
    }

    fn expose_ports(&self) -> &[u16] {
        &self.exposed_ports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImageExt;

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
