use std::collections::HashMap;
use testcontainers::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "minio/minio";
const TAG: &str = "RELEASE.2022-02-07T08-17-33Z";

const DIR: &str = "/data";
const CONSOLE_ADDRESS: &str = ":9001";

#[derive(Debug)]
pub struct MinIO {
    env_vars: HashMap<String, String>,
}

impl Default for MinIO {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "MINIO_CONSOLE_ADDRESS".to_owned(),
            CONSOLE_ADDRESS.to_owned(),
        );

        Self { env_vars }
    }
}

#[derive(Debug, Clone)]
pub struct MinIOServerArgs {
    pub dir: String,
    pub certs_dir: Option<String>,
    pub json_log: bool,
}

impl Default for MinIOServerArgs {
    fn default() -> Self {
        Self {
            dir: DIR.to_owned(),
            certs_dir: None,
            json_log: false,
        }
    }
}

impl ImageArgs for MinIOServerArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        let mut args = vec!["server".to_owned(), self.dir.to_owned()];

        if let Some(ref certs_dir) = self.certs_dir {
            args.push("--certs-dir".to_owned());
            args.push(certs_dir.to_owned())
        }

        if self.json_log {
            args.push("--json".to_owned());
        }

        Box::new(args.into_iter())
    }
}

impl Image for MinIO {
    type Args = MinIOServerArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("API:")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::minio;
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_s3::{Client, Endpoint};
    use aws_types::Credentials;
    use testcontainers::clients;

    #[tokio::test]
    async fn minio_buckets() {
        let docker = clients::Cli::default();
        let minio = minio::MinIO::default();
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
        let endpoint_uri = format!("http://127.0.0.1:{host_port}");
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
}
