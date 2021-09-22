use crate::{core::WaitFor, Image};

const NAME: &str = "softwaremill/elasticmq";
const DEFAULT_TAG: &str = "0.14.6";

#[derive(Debug, Default, Clone)]
pub struct ElasticMqArgs;

impl IntoIterator for ElasticMqArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct ElasticMq {
    tag: String,
    arguments: ElasticMqArgs,
}

impl Default for ElasticMq {
    fn default() -> Self {
        ElasticMq {
            tag: DEFAULT_TAG.to_string(),
            arguments: ElasticMqArgs {},
        }
    }
}

impl Image for ElasticMq {
    type Args = ElasticMqArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started SQS rest server")]
    }
}
