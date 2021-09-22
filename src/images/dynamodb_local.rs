use crate::{core::WaitFor, Image};

const NAME: &str = "amazon/dynamodb-local";
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
}

impl Default for DynamoDb {
    fn default() -> Self {
        DynamoDb {
            tag: DEFAULT_TAG.to_string(),
        }
    }
}

impl Image for DynamoDb {
    type Args = DynamoDbArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout(
                "Initializing DynamoDB Local with the following configuration",
            ),
            WaitFor::millis(DEFAULT_WAIT),
        ]
    }
}
