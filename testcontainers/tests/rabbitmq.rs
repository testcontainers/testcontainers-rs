use futures::StreamExt;
use lapin::{
    options::{
        BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use std::time::Duration;
use testcontainers::{clients, images::rabbitmq};
use tokio_amqp::LapinTokioExt;

#[tokio::test]
async fn rabbitmq_produce_and_consume_messages() {
    let _ = pretty_env_logger::try_init().unwrap();
    let docker = clients::Cli::default();
    let rabbit_node = docker.run(rabbitmq::RabbitMq::default());

    let amqp_url = format!("amqp://127.0.0.1:{}", rabbit_node.get_host_port_ipv4(5672));

    let connection = Connection::connect(
        amqp_url.as_str(),
        ConnectionProperties::default().with_tokio(),
    )
    .await
    .unwrap();

    let channel = connection.create_channel().await.unwrap();

    assert!(channel.status().connected());

    channel
        .exchange_declare(
            "test_exchange",
            ExchangeKind::Topic,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    let queue = channel
        .queue_declare(
            "test_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    channel
        .queue_bind(
            queue.name().as_str(),
            "test_exchange",
            "#",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    let mut consumer = channel
        .basic_consume(
            queue.name().as_str(),
            "test_consumer_tag",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    channel
        .basic_publish(
            "test_exchange",
            "routing-key",
            BasicPublishOptions::default(),
            b"Test Payload".to_vec(),
            BasicProperties::default(),
        )
        .await
        .unwrap();

    let consumed = tokio::time::timeout(Duration::from_secs(10), consumer.next())
        .await
        .unwrap()
        .unwrap();

    let (_, delivery) = consumed.expect("Failed to consume delivery!");
    assert_eq!(
        String::from_utf8(delivery.data.clone()).unwrap(),
        "Test Payload"
    );
    assert_eq!(delivery.exchange.as_str(), "test_exchange");
    assert_eq!(delivery.routing_key.as_str(), "routing-key");
}
