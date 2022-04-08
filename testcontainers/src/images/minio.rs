use crate::{core::WaitFor, Image, ImageArgs};
use std::collections::HashMap;

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
