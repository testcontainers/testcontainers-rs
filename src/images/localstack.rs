use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const NAME: &str = "localstack/localstack";
const TAG: &str = "latest";

#[derive(Debug)]
pub struct LocalStack {
    env_vars: HashMap<String, String>,
    tag: String,
}

impl LocalStack {
    pub fn new(env_vars: HashMap<String, String>, tag: String) -> Self {
        LocalStack { env_vars, tag }
    }

    pub fn with_edge_port(mut self, edge_port: String) -> Self {
        self.env_vars
            .insert("EDGE_PORT".to_owned(), edge_port.to_owned());
        self
    }

    pub fn with_services(mut self, services: String) -> Self {
        self.env_vars
            .insert("SERVICES".to_owned(), services.to_owned());
        self
    }
}

impl Default for LocalStack {
    fn default() -> Self {
        Self {
            env_vars: Default::default(),
            tag: TAG.to_string(),
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
        vec![WaitFor::message_on_stdout("Ready.")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}
