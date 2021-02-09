use crate::{core::WaitFor, Image};

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
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Public node URL:")]
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
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
