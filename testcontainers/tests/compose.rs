#![cfg(feature = "compose")]

use futures::StreamExt;
use log::info;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    ClientConfig, Message,
};
use std::time::Duration;
use testcontainers::{
    compose::{DockerCompose, StopMode},
    core::WaitFor,
};

/// Testing a simple docker compose file
#[test]
fn compose_simple() {
    let _ = pretty_env_logger::try_init();

    let mut dc = DockerCompose::builder("./tests/compose/simple/docker-compose.yml")
        .wait("web1", WaitFor::message_on_stdout("server is ready"))
        .inherit_io()
        .build();

    dc.up();
    dc.block_until_ready();

    let port = dc.get_mapped_port("web1", 80).unwrap_or_default();
    assert_eq!(port, 8081);
}

/// Testing with a docker compose profile
#[test]
fn compose_simple_with_profile() {
    let _ = pretty_env_logger::try_init();

    let mut dc = DockerCompose::builder("./tests/compose/profiles/docker-compose.yml")
        .wait("web1", WaitFor::message_on_stdout("server is ready"))
        .wait("web2", WaitFor::message_on_stdout("server is ready"))
        .wait("web3", WaitFor::message_on_stdout("server is ready"))
        .profiles(["test1", "test2"])
        .build();

    dc.up();
    dc.block_until_ready();

    let port = dc.get_mapped_port("web1", 80).unwrap_or_default();
    assert_eq!(port, 8091);

    let port = dc.get_mapped_port("web2", 80).unwrap_or_default();
    assert_eq!(port, 8092);

    let port = dc.get_mapped_port("web3", 80).unwrap_or_default();
    assert_eq!(port, 8093);
}

/// Testing with Elastic search
/// See <https://www.elastic.co/guide/en/elasticsearch/reference/current/docker.html#docker-compose-file>
#[test]
fn test_elastic() {
    let _ = pretty_env_logger::try_init();

    let mut dc = DockerCompose::builder("./tests/compose/elastic/docker-compose.yml")
        .env("ELASTIC_PASSWORD", "p@ssw0rd!")
        .env("KIBANA_PASSWORD", "p@ssw0rd!")
        .env("STACK_VERSION", "8.4.1")
        .env("CLUSTER_NAME", "docker-cluster")
        .env("LICENSE", "basic")
        .env("ES_PORT", "9200")
        .env("KIBANA_PORT", "5601")
        .env("MEM_LIMIT", "1073741824")
        .wait("es01", WaitFor::Healthcheck)
        .wait("kibana", WaitFor::Healthcheck)
        // you can display docker-compose output for debugging purpose
        // .inherit_io()
        .build();

    dc.up();
    dc.block_until_ready();

    let port = dc.get_mapped_port("es01", 9200).unwrap();
    assert_eq!(port, 9200);

    let port = dc.get_mapped_port("kibana", 5601).unwrap();
    assert_eq!(port, 5601);
}

/// Testing with Kafka `cp-all-in-one-community` and `cp-all-in-one-kraft`
/// See<https://github.com/confluentinc/cp-all-in-one>
///
/// As these config use explicit container name, we need to cleanup the containers
/// So we call the `DockerCompose::rm` function to remove stopped container
#[tokio::test]
async fn compose_kafka_cp_all_in_one_community() {
    let _ = pretty_env_logger::try_init();

    info!("🧪 Testing cp-all-in-one-community");
    let mut dc = build_cp_all_in_one_community();
    dc.up();
    dc.block_until_ready();
    test_kafka(&mut dc).await;
    std::mem::drop(dc); // stop and remove containers

    info!("🧪 Testing cp-all-in-one-kraft");
    let mut dc = build_cp_all_in_one_kraft();
    dc.up();
    dc.block_until_ready();
    test_kafka(&mut dc).await;
}

fn build_cp_all_in_one_community() -> DockerCompose {
    DockerCompose::builder("./tests/compose/cp-all-in-one-community/docker-compose.yml")
        .wait(
            "broker",
            WaitFor::message_on_stdout("Checking need to trigger auto leader balancing"),
        )
        .wait(
            "schema-registry",
            WaitFor::message_on_stdout("Server started, listening for requests..."),
        )
        .wait(
            "rest-proxy",
            WaitFor::message_on_stdout("Server started, listening for requests..."),
        )
        // IMPORTANT, we need to remove containers while dropping the `DockerCompose`
        .stop_with(StopMode::StopAndRemove)
        // you can display docker-compose output for debugging purpose
        // .inherit_io()
        .build()
}

fn build_cp_all_in_one_kraft() -> DockerCompose {
    DockerCompose::builder("./tests/compose/cp-all-in-one-kraft/docker-compose.yml")
        .wait("broker", WaitFor::message_on_stdout("Kafka Server started"))
        .wait(
            "schema-registry",
            WaitFor::message_on_stdout("Server started, listening for requests..."),
        )
        .wait(
            "rest-proxy",
            WaitFor::message_on_stdout("Server started, listening for requests..."),
        )
        // IMPORTANT, we need to remove containers while dropping the `DockerCompose`
        .stop_with(StopMode::StopAndRemove)
        // you can display docker-compose output for debugging purpose
        // .inherit_io()
        .build()
}

async fn test_kafka(dc: &mut DockerCompose) {
    let port = dc.get_mapped_port("broker", 9092);
    let bootstrap_servers = format!("127.0.0.1:{}", port.unwrap());
    test_produce_and_consume_messages(&bootstrap_servers).await;

    let port = dc.get_mapped_port("rest-proxy", 8082);
    let rest_proxy = format!("http://127.0.0.1:{}", port.unwrap());
    test_list_topics(&rest_proxy).await;
}

async fn test_produce_and_consume_messages(bootstrap_servers: &str) {
    let producer = ClientConfig::new()
        .set("bootstrap.servers", bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create::<FutureProducer>()
        .expect("Failed to create Kafka FutureProducer");

    let consumer = ClientConfig::new()
        .set("group.id", "testcontainer-rs")
        .set("bootstrap.servers", bootstrap_servers)
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

async fn test_list_topics(rest_proxy: &str) {
    let client = reqwest::Client::builder().build().unwrap();
    let url = format!("{rest_proxy}/topics");
    let result = client.get(url).send().await.unwrap();
    let content = result.text().await.unwrap();
    let topics = serde_json::from_str::<Vec<String>>(&content).unwrap();
    assert!(topics.iter().any(|topic| topic == "test-topic"));
}
