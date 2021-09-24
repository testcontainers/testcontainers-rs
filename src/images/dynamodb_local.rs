use crate::{core::WaitFor, Image};

const NAME: &str = "amazon/dynamodb-local";
const TAG: &str = "latest";
const DEFAULT_WAIT: u64 = 2000;

#[derive(Default, Debug)]
pub struct DynamoDb;

impl Image for DynamoDb {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
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
