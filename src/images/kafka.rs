use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "confluentinc/cp-kafka";
const DEFAULT_TAG: &str = "6.1.1";

pub const KAFKA_PORT: u16 = 9093;
const ZOOKEEPER_PORT: u16 = 2181;

#[derive(Clone, Debug, Default)]
pub struct KafkaArgs;

impl IntoIterator for KafkaArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            "/bin/bash".to_owned(),
            "-c".to_owned(),
            format!(
                r#"
echo 'clientPort={}' > zookeeper.properties;
echo 'dataDir=/var/lib/zookeeper/data' >> zookeeper.properties;
echo 'dataLogDir=/var/lib/zookeeper/log' >> zookeeper.properties;
zookeeper-server-start zookeeper.properties &
. /etc/confluent/docker/bash-config &&
/etc/confluent/docker/configure &&
/etc/confluent/docker/launch"#,
                ZOOKEEPER_PORT
            ),
        ]
        .into_iter()
    }
}

#[derive(Debug)]
pub struct Kafka {
    arguments: KafkaArgs,
    env_vars: HashMap<String, String>,
    tag: String,
}

impl Kafka {
    pub fn with_tag<T: Into<String>>(self, tag: T) -> Self {
        Self {
            tag: tag.into(),
            ..self
        }
    }
}

impl Default for Kafka {
    fn default() -> Self {
        let mut env_vars = HashMap::new();

        env_vars.insert(
            "KAFKA_ZOOKEEPER_CONNECT".to_owned(),
            format!("localhost:{}", ZOOKEEPER_PORT),
        );
        env_vars.insert(
            "KAFKA_LISTENERS".to_owned(),
            format!("PLAINTEXT://0.0.0.0:{},BROKER://0.0.0.0:9092", KAFKA_PORT),
        );
        env_vars.insert(
            "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP".to_owned(),
            "BROKER:PLAINTEXT,PLAINTEXT:PLAINTEXT".to_owned(),
        );
        env_vars.insert(
            "KAFKA_INTER_BROKER_LISTENER_NAME".to_owned(),
            "BROKER".to_owned(),
        );
        env_vars.insert(
            "KAFKA_ADVERTISED_LISTENERS".to_owned(),
            format!(
                "PLAINTEXT://localhost:{},BROKER://localhost:9092",
                KAFKA_PORT
            ),
        );
        env_vars.insert("KAFKA_BROKER_ID".to_owned(), "1".to_owned());
        env_vars.insert(
            "KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR".to_owned(),
            "1".to_owned(),
        );

        Self {
            arguments: KafkaArgs::default(),
            env_vars,
            tag: DEFAULT_TAG.to_owned(),
        }
    }
}

impl Image for Kafka {
    type Args = KafkaArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Creating new log file")]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        Self { arguments, ..self }
    }
}
