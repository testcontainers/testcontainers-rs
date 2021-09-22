use crate::{core::WaitFor, Image};

const NAME: &str = "parity/parity";
const DEFAULT_TAG: &str = "v2.5.0";

#[derive(Debug)]
pub struct ParityEthereum {
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
            tag: DEFAULT_TAG.to_string(),
        }
    }
}

impl Image for ParityEthereum {
    type Args = ParityEthereumArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Public node URL:")]
    }
}
