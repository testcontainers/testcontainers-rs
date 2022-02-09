use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const NAME: &str = "docker.elastic.co/elasticsearch/elasticsearch";
const TAG: &str = "7.16.1";

#[derive(Debug)]
pub struct ElasticSearch {
    env_vars: HashMap<String, String>,
    tag: String,
}

impl Default for ElasticSearch {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("discovery.type".to_owned(), "single-node".to_owned());
        ElasticSearch {
            env_vars,
            tag: TAG.to_owned(),
        }
    }
}

impl Image for ElasticSearch {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("[YELLOW] to [GREEN]")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![9200, 9300]
    }
}
