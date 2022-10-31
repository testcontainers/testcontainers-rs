use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, Endpoint};
use aws_types::Credentials;
use testcontainers::*;

#[tokio::test]
async fn minio_buckets() {
    let docker = clients::Cli::default();
    let minio = images::minio::MinIO::default();
    let node = docker.run(minio);

    let host_port = node.get_host_port_ipv4(9000);

    let client = build_s3_client(host_port).await;

    let bucket_name = "test-bucket";

    client
        .create_bucket()
        .bucket(bucket_name)
        .send()
        .await
        .expect("Failed to create test bucket");

    let buckets = client
        .list_buckets()
        .send()
        .await
        .expect("Failed to get list of buckets")
        .buckets
        .unwrap();
    assert_eq!(1, buckets.len());
    assert_eq!(bucket_name, buckets[0].name.as_ref().unwrap());
}

async fn build_s3_client(host_port: u16) -> Client {
    let endpoint_uri = format!("http://127.0.0.1:{}", host_port);
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let creds = Credentials::new("minioadmin", "minioadmin", None, None, "test");

    // Default MinIO credentials (Can be overridden by ENV container variables)
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
