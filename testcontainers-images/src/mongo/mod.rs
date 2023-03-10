use testcontainers::{core::WaitFor, Image};

const NAME: &str = "mongo";
const TAG: &str = "5.0.6";

#[derive(Default, Debug)]
pub struct Mongo;

impl Image for Mongo {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Waiting for connections")]
    }
}

#[cfg(test)]
mod tests {
    use crate::mongo;
    use mongodb::*;
    use testcontainers::clients;

    #[tokio::test]
    async fn mongo_fetch_document() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(mongo::Mongo::default());
        let host_port = node.get_host_port_ipv4(27017);
        let url = format!("mongodb://127.0.0.1:{host_port}/");

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
}
