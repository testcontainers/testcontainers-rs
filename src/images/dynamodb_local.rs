use crate::{Container, Docker, Image, WaitForMessage};
use std::{collections::HashMap, env::var, thread::sleep, time::Duration};

const ADDITIONAL_SLEEP_PERIOD: &str = "DYNAMODB_ADDITIONAL_SLEEP_PERIOD";
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
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Initializing DynamoDB Local with the following configuration")
            .unwrap();

        let additional_sleep_period = var(ADDITIONAL_SLEEP_PERIOD)
            .map(|value| value.parse().unwrap_or(DEFAULT_WAIT))
            .unwrap_or(DEFAULT_WAIT);

        let sleep_period = Duration::from_millis(additional_sleep_period);

        log::trace!(
            "Waiting for an additional {:?} for container {}.",
            sleep_period,
            container.id()
        );

        sleep(sleep_period)
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
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
