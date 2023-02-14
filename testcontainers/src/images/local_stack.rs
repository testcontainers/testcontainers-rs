use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const NAME: &str = "localstack/localstack";
const TAG: &str = "1.4";
const PORT: u16 = 4566;

#[derive(Debug)]
pub struct LocalStack {
    env_vars: HashMap<String, String>,
    volumes: HashMap<String, String>,
    tag: String,
}

impl Default for LocalStack {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("DEBUG".to_owned(), "1".to_owned());
        env_vars.insert("PORT_WEB_UI".to_owned(), "8080".to_owned());
        env_vars.insert("LAMBDA_EXECUTOR".to_owned(), "docker".to_owned());
        env_vars.insert("DOCKER_HOST".to_owned(), "unix:///var/run/docker.sock".to_owned());
        env_vars.insert("DATA_DIR".to_owned(), "/tmp/localstack/data".to_owned());

        let mut volumes = HashMap::new();
        volumes.insert("/var/run/docker.sock".to_owned(), "/var/run/docker.sock".to_owned());
        LocalStack {
            env_vars,
            tag: TAG.to_owned(),
            volumes
        }
    }
}

impl Image for LocalStack {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout( format!("Ready."))]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![PORT]
    }
    fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.volumes.iter())
    }

}
