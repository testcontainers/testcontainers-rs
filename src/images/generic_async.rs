use crate::core::Port;
use crate::core::{WaitErrorAsync, WaitForMessageAsync};
use crate::{ContainerAsync, DockerAsync, ImageAsync};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum WaitForAsync {
    Nothing,
    LogMessage { message: String, stream: Stream },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stream {
    StdOut,
    StdErr,
}

impl WaitForAsync {
    pub fn message_on_stdout<S: Into<String>>(message: S) -> WaitForAsync {
        WaitForAsync::LogMessage {
            message: message.into(),
            stream: Stream::StdOut,
        }
    }

    pub fn message_on_stderr<S: Into<String>>(message: S) -> WaitForAsync {
        WaitForAsync::LogMessage {
            message: message.into(),
            stream: Stream::StdErr,
        }
    }

    async fn wait<D: DockerAsync, I: ImageAsync>(
        &self,
        container: &ContainerAsync<'_, D, I>,
    ) -> Result<(), WaitErrorAsync> {
        match self {
            WaitForAsync::Nothing => Ok(()),
            WaitForAsync::LogMessage { message, stream } => match stream {
                Stream::StdOut => {
                    container
                        .logs()
                        .await
                        .stdout
                        .wait_for_message_async(message)
                        .await
                }
                Stream::StdErr => {
                    container
                        .logs()
                        .await
                        .stderr
                        .wait_for_message_async(message)
                        .await
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct GenericImageAsync {
    descriptor: String,
    arguments: Vec<String>,
    volumes: HashMap<String, String>,
    env_vars: HashMap<String, String>,
    ports: Option<Vec<Port>>,
    wait_for: WaitForAsync,
    entrypoint: Option<String>,
}

impl Default for GenericImageAsync {
    fn default() -> Self {
        Self {
            descriptor: "".to_owned(),
            arguments: vec![],
            volumes: HashMap::new(),
            env_vars: HashMap::new(),
            wait_for: WaitForAsync::Nothing,
            ports: None,
            entrypoint: None,
        }
    }
}

impl GenericImageAsync {
    pub fn new<S: Into<String>>(descriptor: S) -> GenericImageAsync {
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

    pub fn with_mapped_port<P: Into<Port>>(mut self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());
        self.ports = Some(ports);
        self
    }

    pub fn with_wait_for(mut self, wait_for: WaitForAsync) -> Self {
        self.wait_for = wait_for;
        self
    }
}

#[async_trait]
impl ImageAsync for GenericImageAsync {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = str;

    fn descriptor(&self) -> String {
        self.descriptor.to_owned()
    }

    async fn wait_until_ready<D: DockerAsync>(&self, container: &ContainerAsync<'_, D, Self>) {
        self.wait_for.wait(container).await.unwrap();
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

    fn ports(&self) -> Option<Vec<Port>> {
        self.ports.clone()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }

    fn with_entrypoint(self, entrypoint: &Self::EntryPoint) -> Self {
        Self {
            entrypoint: Some(entrypoint.to_string()),
            ..self
        }
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
        let image = GenericImageAsync::new("hello")
            .with_env_var("one-key", "one-value")
            .with_env_var("two-key", "two-value");

        let env_vars = image.env_vars();
        assert_eq!(2, env_vars.len());
        assert_eq!("one-value", env_vars.get("one-key").unwrap());
        assert_eq!("two-value", env_vars.get("two-key").unwrap());
    }

    #[test]
    fn should_return_ports() {
        let mut image = GenericImageAsync::new("hello");
        assert!(image.ports().is_none());

        image = image
            .with_mapped_port((123, 456))
            .with_mapped_port((555, 888));

        assert_eq!(
            vec![
                Port {
                    local: 123,
                    internal: 456
                },
                Port {
                    local: 555,
                    internal: 888
                },
            ],
            image.ports().unwrap()
        );
    }
}
