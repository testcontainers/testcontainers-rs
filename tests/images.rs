use bitcoincore_rpc::RpcApi;
use mongodb::{bson, Client};
use redis::Commands;
use rusoto_core::{HttpClient, Region};
use rusoto_credential::StaticProvider;
use rusoto_dynamodb::{
    AttributeDefinition, CreateTableInput, DynamoDb, DynamoDbClient, KeySchemaElement,
    ProvisionedThroughput,
};
use rusoto_sqs::{ListQueuesRequest, Sqs, SqsClient};
use spectral::prelude::*;
use std::time::Duration;
use zookeeper::{Acl, CreateMode, ZooKeeper};

use testcontainers::{core::WaitFor, *};

#[test]
fn coblox_bitcoincore_getnewaddress() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::coblox_bitcoincore::BitcoinCore::default());

    let client = {
        let host_port = node.get_host_port(18443);

        let url = format!("http://localhost:{}", host_port);

        let auth = node.image().auth();

        bitcoincore_rpc::Client::new(
            url,
            bitcoincore_rpc::Auth::UserPass(auth.username().to_owned(), auth.password().to_owned()),
        )
        .unwrap()
    };

    assert_that(&client.create_wallet("miner", None, None, None, None)).is_ok();

    assert_that(&client.get_new_address(None, None)).is_ok();
}

#[test]
fn parity_parity_net_version() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::parity_parity::ParityEthereum::default());
    let host_port = node.get_host_port(8545);

    let response = reqwest::blocking::Client::new()
        .post(&format!("http://localhost:{}", host_port))
        .body(
            json::object! {
                "jsonrpc" => "2.0",
                "method" => "net_version",
                "params" => json::array![],
                "id" => 1
            }
            .dump(),
        )
        .header("content-type", "application/json")
        .send()
        .unwrap();

    let response = response.text().unwrap();
    let response = json::parse(&response).unwrap();

    assert_eq!(response["result"], "17");
}

#[test]
fn trufflesuite_ganachecli_listaccounts() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::trufflesuite_ganachecli::GanacheCli::default());
    let host_port = node.get_host_port(8545);

    let response = reqwest::blocking::Client::new()
        .post(&format!("http://localhost:{}", host_port))
        .body(
            json::object! {
                "jsonrpc" => "2.0",
                "method" => "net_version",
                "params" => json::array![],
                "id" => 1
            }
            .dump(),
        )
        .header("content-type", "application/json")
        .send()
        .unwrap();

    let response = response.text().unwrap();
    let response = json::parse(&response).unwrap();

    assert_eq!(response["result"], "42");
}

#[tokio::test]
async fn dynamodb_local_create_table() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::dynamodb_local::DynamoDb::default());
    let host_port = node.get_host_port(8000);

    let create_tables_input = CreateTableInput {
        table_name: "books".to_string(),
        key_schema: vec![KeySchemaElement {
            key_type: "HASH".to_string(),
            attribute_name: "title".to_string(),
        }],
        attribute_definitions: vec![AttributeDefinition {
            attribute_name: "title".to_string(),
            attribute_type: "S".to_string(),
        }],
        provisioned_throughput: Some(ProvisionedThroughput {
            read_capacity_units: 5,
            write_capacity_units: 5,
        }),
        ..Default::default()
    };

    let dynamodb = build_dynamodb_client(host_port);
    let result = dynamodb.create_table(create_tables_input).await;
    assert_that(&result).is_ok();
}

fn build_dynamodb_client(host_port: u16) -> DynamoDbClient {
    let credentials_provider =
        StaticProvider::new("fakeKey".to_string(), "fakeSecret".to_string(), None, None);

    let dispatcher = HttpClient::new().expect("could not create http client");

    let region = Region::Custom {
        name: "dynamodb-local".to_string(),
        endpoint: format!("http://localhost:{}", host_port),
    };

    DynamoDbClient::new_with(dispatcher, credentials_provider, region)
}

#[test]
fn redis_fetch_an_integer() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::redis::Redis::default());
    let host_port = node.get_host_port(6379);
    let url = format!("redis://localhost:{}", host_port);

    let client = redis::Client::open(url.as_ref()).unwrap();
    let mut con = client.get_connection().unwrap();

    con.set::<_, _, ()>("my_key", 42).unwrap();
    let result: i64 = con.get("my_key").unwrap();
    assert_eq!(42, result);
}

#[tokio::test]
async fn mongo_fetch_document() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::mongo::Mongo::default());
    let host_port = node.get_host_port(27017);
    let url = format!("mongodb://localhost:{}/", host_port);

    let client: Client = Client::with_uri_str(&url).await.unwrap();
    let db = client.database("some_db");
    let coll = db.collection("some-coll");

    let insert_one_result = coll.insert_one(bson::doc! { "x": 42 }, None).await.unwrap();
    assert!(!insert_one_result
        .inserted_id
        .as_object_id()
        .unwrap()
        .to_hex()
        .is_empty());

    let find_one_result: bson::Document = coll
        .find_one(bson::doc! { "x": 42 }, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(42, find_one_result.get_i32("x").unwrap())
}

#[tokio::test]
async fn sqs_list_queues() {
    let docker = clients::Cli::default();
    let node = docker.run(images::elasticmq::ElasticMq::default());
    let host_port = node.get_host_port(9324);
    let client = build_sqs_client(host_port);

    let request = ListQueuesRequest::default();
    let result = client.list_queues(request).await.unwrap();
    assert!(result.queue_urls.is_none());
}

#[test]
fn generic_image() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();

    let db = "postgres-db-test";
    let user = "postgres-user-test";
    let password = "postgres-password-test";

    let generic_postgres = images::generic::GenericImage::new("postgres:9.6-alpine")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_DB", db)
        .with_env_var("POSTGRES_USER", user)
        .with_env_var("POSTGRES_PASSWORD", password);

    let node = docker.run(generic_postgres);

    let connection_string = &format!(
        "postgres://{}:{}@localhost:{}/{}",
        user,
        password,
        node.get_host_port(5432),
        db
    );
    let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

    let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let first_column: i32 = first_row.get(0);
    assert_eq!(first_column, 2);
}

#[test]
fn generic_image_with_custom_entrypoint() {
    let docker = clients::Cli::default();
    let msg = WaitFor::message_on_stdout("server is ready");

    let generic = images::generic::GenericImage::new("tumdum/simple_web_server:latest")
        .with_wait_for(msg.clone());

    let node = docker.run(generic);
    let port = node.get_host_port(80);
    assert_eq!(
        "foo",
        reqwest::blocking::get(&format!("http://127.0.0.1:{}", port))
            .unwrap()
            .text()
            .unwrap()
    );

    let generic = images::generic::GenericImage::new("tumdum/simple_web_server:latest")
        .with_wait_for(msg)
        .with_entrypoint("/bar");

    let node = docker.run(generic);
    let port = node.get_host_port(80);
    assert_eq!(
        "bar",
        reqwest::blocking::get(&format!("http://127.0.0.1:{}", port))
            .unwrap()
            .text()
            .unwrap()
    );
}

fn build_sqs_client(host_port: u16) -> SqsClient {
    let dispatcher = HttpClient::new().expect("could not create http client");
    let credentials_provider =
        StaticProvider::new("fakeKey".to_string(), "fakeSecret".to_string(), None, None);
    let region = Region::Custom {
        name: "sqs-local".to_string(),
        endpoint: format!("http://localhost:{}", host_port),
    };

    SqsClient::new_with(dispatcher, credentials_provider, region)
}

#[test]
fn postgres_one_plus_one() {
    let docker = clients::Cli::default();
    let postgres_image = images::postgres::Postgres::default();
    let node = docker.run(postgres_image);

    let connection_string = &format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        node.get_host_port(5432)
    );
    let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

    let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let first_column: i32 = first_row.get(0);
    assert_eq!(first_column, 2);
}

#[test]
fn postgres_one_plus_one_with_custom_mapped_port() {
    let _ = pretty_env_logger::try_init();
    let free_local_port = free_local_port().unwrap();

    let docker = clients::Cli::default();
    let run_args = RunArgs::default().with_mapped_port((free_local_port, 5432));
    let _node = docker.run_with_args(images::postgres::Postgres::default(), run_args);

    let mut conn = postgres::Client::connect(
        &format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            free_local_port
        ),
        postgres::NoTls,
    )
    .unwrap();
    let rows = conn.query("SELECT 1+1 AS result;", &[]).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get::<_, i32>("result"), 2);
}

#[test]
fn postgres_custom_version() {
    let docker = clients::Cli::default();
    let postgres_image = images::postgres::Postgres::default().with_version(13);
    let node = docker.run(postgres_image);

    let connection_string = &format!(
        "postgres://postgres:postgres@localhost:{}/postgres",
        node.get_host_port(5432)
    );
    let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

    let rows = conn.query("SELECT version()", &[]).unwrap();
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let first_column: String = first_row.get(0);
    assert!(first_column.contains("13"));
}

/// Returns an available localhost port
pub fn free_local_port() -> Option<u16> {
    let socket = std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 0);
    std::net::TcpListener::bind(socket)
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .ok()
}

#[test]
#[ignore]
fn zookeeper_check_directories_existence() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Cli::default();
    let image = images::zookeeper::Zookeeper::default();
    let node = docker.run(image);

    let host_port = node.get_host_port(2181);
    let zk_urls = format!("localhost:{}", host_port);
    let zk = ZooKeeper::connect(&*zk_urls, Duration::from_secs(15), |_| ()).unwrap();

    zk.create(
        "/test",
        vec![1, 2],
        Acl::open_unsafe().clone(),
        CreateMode::Ephemeral,
    )
    .unwrap();

    assert!(matches!(zk.exists("/test", false).unwrap(), Some(_)));
    assert!(matches!(zk.exists("/test2", false).unwrap(), None));
}

#[test]
#[ignore]
fn orientdb_exists_database() {
    let docker = clients::Cli::default();
    let orientdb_image = images::orientdb::OrientDb::default();
    let node = docker.run(orientdb_image);

    let client =
        orientdb_client::OrientDB::connect(("localhost", node.get_host_port(2424))).unwrap();

    let exists = client
        .exist_database(
            "orientdb_exists_database",
            "root",
            "root",
            orientdb_client::DatabaseType::Memory,
        )
        .unwrap();

    assert!(!exists);
}
