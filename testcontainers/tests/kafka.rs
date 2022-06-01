use futures::StreamExt;
use rdkafka::{
    config::ClientConfig,
    consumer::{stream_consumer::StreamConsumer, Consumer},
    producer::{FutureProducer, FutureRecord},
    Message,
};
use std::time::Duration;
use testcontainers::{clients, images::kafka};

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
        .map(|i| format!("Message {}", i))
        .collect();

    for (i, message) in expected.iter().enumerate() {
        producer
            .send(
                FutureRecord::to(topic)
                    .payload(message)
                    .key(&format!("Key {}", i)),
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
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
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
