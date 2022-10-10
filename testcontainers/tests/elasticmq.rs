use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sqs::{Client, Endpoint};
use aws_types::Credentials;
use testcontainers::*;

#[tokio::test]
async fn sqs_list_queues() {
    let docker = clients::Cli::default();
    let node = docker.run(images::elasticmq::ElasticMq::default());
    let host_port = node.get_host_port_ipv4(9324);
    let client = build_sqs_client(host_port).await;

    let result = client.list_queues().send().await.unwrap();
    assert!(result.queue_urls.is_none());
}

async fn build_sqs_client(host_port: u16) -> Client {
    let endpoint_uri = format!("http://127.0.0.1:{}", host_port);
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let creds = Credentials::new("fakeKey", "fakeSecret", None, None, "test");

    let shared_config = aws_config::from_env()
        .region(region_provider)
        .endpoint_resolver(Endpoint::immutable(
            endpoint_uri.parse().expect("valid URI"),
        ))
        .credentials_provider(creds)
        .load()
        .await;

    Client::new(&shared_config)
}
