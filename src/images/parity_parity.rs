use crate::{Container, Docker, Image, WaitForMessage};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "parity/parity";
const DEFAULT_TAG: &str = "v2.5.0";

#[derive(Debug)]
pub struct ParityEthereum {
    arguments: ParityEthereumArgs,
    tag: String,
}

#[derive(Default, Debug, Clone)]
pub struct ParityEthereumArgs {}

impl IntoIterator for ParityEthereumArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            "--config=dev".to_string(),
            "--jsonrpc-apis=all".to_string(),
            "--unsafe-expose".to_string(),
            "--tracing=on".to_string(),
        ]
        .into_iter()
    }
}

impl Default for ParityEthereum {
    fn default() -> Self {
        ParityEthereum {
            arguments: ParityEthereumArgs {},
            tag: DEFAULT_TAG.to_string(),
        }
    }
}

impl Image for ParityEthereum {
    type Args = ParityEthereumArgs;
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
            .wait_for_message("Public node URL:")
            .unwrap();
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }
}

impl ParityEthereum {
    pub fn with_tag(self, tag_str: &str) -> Self {
        ParityEthereum {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
