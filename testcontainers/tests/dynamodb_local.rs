use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{
    model::{
        AttributeDefinition, KeySchemaElement, KeyType, ProvisionedThroughput, ScalarAttributeType,
    },
    Client, Endpoint,
};
use aws_types::Credentials;
use spectral::prelude::*;
use testcontainers::*;

#[tokio::test]
async fn dynamodb_local_create_table() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::dynamodb_local::DynamoDb::default());
    let host_port = node.get_host_port_ipv4(8000);

    let table_name = "books".to_string();

    let key_schema = KeySchemaElement::builder()
        .attribute_name("title".to_string())
        .key_type(KeyType::Hash)
        .build();

    let attribute_def = AttributeDefinition::builder()
        .attribute_name("title".to_string())
        .attribute_type(ScalarAttributeType::S)
        .build();

    let provisioned_throughput = ProvisionedThroughput::builder()
        .read_capacity_units(10)
        .write_capacity_units(5)
        .build();

    let dynamodb = build_dynamodb_client(host_port).await;
    let create_table_result = dynamodb
        .create_table()
        .table_name(table_name)
        .key_schema(key_schema)
        .attribute_definitions(attribute_def)
        .provisioned_throughput(provisioned_throughput)
        .send()
        .await;
    assert_that(&create_table_result).is_ok();

    let req = dynamodb.list_tables().limit(10);
    let list_tables_result = req.send().await.unwrap();

    assert_eq!(list_tables_result.table_names().unwrap().len(), 1);
}

async fn build_dynamodb_client(host_port: u16) -> Client {
    let endpoint_uri = format!("http://127.0.0.1:{host_port}");
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
