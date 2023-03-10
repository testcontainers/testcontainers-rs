use std::collections::HashMap;
use testcontainers::{
    core::{ContainerState, ExecCommand, WaitFor},
    Image, ImageArgs,
};

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
echo 'clientPort={ZOOKEEPER_PORT}' > zookeeper.properties;
echo 'dataDir=/var/lib/zookeeper/data' >> zookeeper.properties;
echo 'dataLogDir=/var/lib/zookeeper/log' >> zookeeper.properties;
zookeeper-server-start zookeeper.properties &
. /etc/confluent/docker/bash-config &&
/etc/confluent/docker/configure &&
/etc/confluent/docker/launch"#,
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
            format!("localhost:{ZOOKEEPER_PORT}"),
        );
        env_vars.insert(
            "KAFKA_LISTENERS".to_owned(),
            format!("PLAINTEXT://0.0.0.0:{KAFKA_PORT},BROKER://0.0.0.0:9092"),
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
            format!("PLAINTEXT://localhost:{KAFKA_PORT},BROKER://localhost:9092",),
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

#[cfg(test)]
mod tests {
    use crate::kafka;
    use futures::StreamExt;
    use rdkafka::consumer::{Consumer, StreamConsumer};
    use rdkafka::producer::{FutureProducer, FutureRecord};
    use rdkafka::{ClientConfig, Message};
    use std::time::Duration;
    use testcontainers::clients;

    #[tokio::test]
    async fn produce_and_consume_messages() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let kafka_node = docker.run(kafka::Kafka::default());

        let bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT)
        );

        let producer = ClientConfig::new()
            .set("bootstrap.servers", &bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create::<FutureProducer>()
            .expect("Failed to create Kafka FutureProducer");

        let consumer = ClientConfig::new()
            .set("group.id", "testcontainer-rs")
            .set("bootstrap.servers", &bootstrap_servers)
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create::<StreamConsumer>()
            .expect("Failed to create Kafka StreamConsumer");

        let topic = "test-topic";

        let number_of_messages_to_produce = 5_usize;
        let expected: Vec<String> = (0..number_of_messages_to_produce)
            .map(|i| format!("Message {i}"))
            .collect();

        for (i, message) in expected.iter().enumerate() {
            producer
                .send(
                    FutureRecord::to(topic)
                        .payload(message)
                        .key(&format!("Key {i}")),
                    Duration::from_secs(0),
                )
                .await
                .unwrap();
        }

        consumer
            .subscribe(&[topic])
            .expect("Failed to subscribe to a topic");

        let mut message_stream = consumer.stream();
        for produced in expected {
            let borrowed_message =
                tokio::time::timeout(Duration::from_secs(10), message_stream.next())
                    .await
                    .unwrap()
                    .unwrap();

            assert_eq!(
                produced,
                borrowed_message
                    .unwrap()
                    .payload_view::<str>()
                    .unwrap()
                    .unwrap()
            );
        }
    }
}
