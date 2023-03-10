use testcontainers::{core::WaitFor, Image};

const NAME: &str = "rabbitmq";
const TAG: &str = "3.8.22-management";

#[derive(Debug, Default, Clone)]
pub struct RabbitMq;

impl Image for RabbitMq {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "Server startup complete; 4 plugins started.",
        )]
    }
}

#[cfg(test)]
mod tests {
    use crate::rabbitmq;
    use futures::StreamExt;
    use lapin::options::{
        BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    };
    use lapin::types::FieldTable;
    use lapin::{BasicProperties, Connection, ConnectionProperties, ExchangeKind};
    use std::time::Duration;
    use testcontainers::clients;
    use tokio_amqp::LapinTokioExt;

    #[tokio::test]
    async fn rabbitmq_produce_and_consume_messages() {
        pretty_env_logger::try_init().unwrap();
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
}
