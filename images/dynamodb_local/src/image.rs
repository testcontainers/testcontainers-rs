use std::{env::var, thread::sleep, time::Duration};
use tc_core::{Image, Container, Docker, WaitForMessage};

const ADDITIONAL_SLEEP_PERIOD: &'static str = "DYNAMODB_ADDITIONAL_SLEEP_PERIOD";
const DEFAULT_WAIT: u64 = 2000;
const CONTAINER_IDENTIFIER: &'static str = "amazon/dynamodb-local";
const DEFAULT_TAG: &'static str = "latest";


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
    arguments: DynamoDbArgs
}

impl Default for DynamoDb {
    fn default() -> Self {
        DynamoDb {
            tag: DEFAULT_TAG.to_string(),
            arguments: DynamoDbArgs {}
        }
    }
}

impl Image for DynamoDb {
    type Args = DynamoDbArgs;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Initializing DynamoDB Local with the following configuration")
            .unwrap();

        let additional_sleep_period = var(ADDITIONAL_SLEEP_PERIOD)
            .map(|value| value.parse().unwrap_or(DEFAULT_WAIT))
            .unwrap_or(DEFAULT_WAIT);

        let sleep_period = Duration::from_millis(additional_sleep_period);

        trace!(
            "Waiting for an additional {:?} for container {}.",
            sleep_period,
            container.id()
        );

        sleep(sleep_period)
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        DynamoDb {
            arguments, ..self
        }
    }

}

impl DynamoDb {
    pub fn with_tag_args(self, tag_str: &str, arguments: <Self as Image>::Args) -> Self {
        DynamoDb{tag: tag_str.to_string(), arguments, ..self}
    }
}



