use crate::{core::WaitFor, Image};

const CONTAINER_IDENTIFIER: &str = "amazon/dynamodb-local";
const DEFAULT_WAIT: u64 = 2000;
const DEFAULT_TAG: &str = "latest";

#[derive(Debug, Default, Clone)]
pub struct DynamoDbArgs;

impl IntoIterator for DynamoDbArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct DynamoDb {
    tag: String,
    arguments: DynamoDbArgs,
}

impl Default for DynamoDb {
    fn default() -> Self {
        DynamoDb {
            tag: DEFAULT_TAG.to_string(),
            arguments: DynamoDbArgs {},
        }
    }
}

impl Image for DynamoDb {
    type Args = DynamoDbArgs;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout(
                "Initializing DynamoDB Local with the following configuration",
            ),
            WaitFor::millis(DEFAULT_WAIT),
        ]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        DynamoDb { arguments, ..self }
    }
}

impl DynamoDb {
    pub fn with_tag(self, tag_str: &str) -> Self {
        DynamoDb {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
