use crate::{
    core::{ContainerState, ExecCommand, WaitFor},
    Image, ImageArgs,
};
use std::collections::HashMap;

const NAME: &str = "confluentinc/cp-kafka";
const TAG: &str = "6.1.1";

pub const KAFKA_PORT: u16 = 9093;
const ZOOKEEPER_PORT: u16 = 2181;

#[derive(Debug, Default, Clone)]
pub struct KafkaArgs;

impl ImageArgs for KafkaArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
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
            .into_iter(),
        )
    }
}

#[derive(Debug)]
pub struct Kafka {
    env_vars: HashMap<String, String>,
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

        Self { env_vars }
    }
}

impl Image for Kafka {
    type Args = KafkaArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Creating new log file")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![KAFKA_PORT]
    }

    fn exec_after_start(&self, cs: ContainerState) -> Vec<ExecCommand> {
        let mut commands = vec![];
        let cmd = format!(
            "kafka-configs --alter --bootstrap-server 0.0.0.0:9092 --entity-type brokers --entity-name 1 --add-config advertised.listeners=[PLAINTEXT://127.0.0.1:{},BROKER://localhost:9092]",
            cs.host_port_ipv4(KAFKA_PORT)
        );
        let ready_conditions = vec![WaitFor::message_on_stdout(
            "Checking need to trigger auto leader balancing",
        )];
        commands.push(ExecCommand {
            cmd,
            ready_conditions,
        });
        commands
    }
}
