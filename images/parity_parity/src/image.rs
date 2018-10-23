use tc_core::{Container, Docker, Image, WaitForMessage};

#[derive(Debug)]
pub struct ParityEthereum {
    arguments: ParityEthereumArgs,
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
        ].into_iter()
    }
}

impl Default for ParityEthereum {
    fn default() -> Self {
        ParityEthereum {
            arguments: ParityEthereumArgs {},
        }
    }
}

impl Image for ParityEthereum {
    type Args = ParityEthereumArgs;

    fn descriptor(&self) -> String {
        "parity/parity:v1.11.11".to_string()
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        container
            .logs()
            .stderr
            .wait_for_message("Public node URL:")
            .unwrap();
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments }
    }
}
