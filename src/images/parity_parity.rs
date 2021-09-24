use crate::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "parity/parity";
const TAG: &str = "v2.5.0";

#[derive(Debug, Default)]
pub struct ParityEthereum;

#[derive(Debug, Default, Clone)]
pub struct ParityEthereumArgs;

impl ImageArgs for ParityEthereumArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
            vec![
                "--config=dev".to_string(),
                "--jsonrpc-apis=all".to_string(),
                "--unsafe-expose".to_string(),
                "--tracing=on".to_string(),
            ]
            .into_iter(),
        )
    }
}

impl Image for ParityEthereum {
    type Args = ParityEthereumArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Public node URL:")]
    }
}
