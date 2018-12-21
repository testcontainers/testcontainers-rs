use tc_core::{Container, Docker, Image, WaitForMessage};

const CONTAINER_IDENTIFIER: &'static str = "softwaremill/elasticmq";
const DEFAULT_TAG: &'static str = "0.14.6";

#[derive(Debug, Default, Clone)]
pub struct ElasticMQArgs;

impl IntoIterator for ElasticMQArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct ElasticMQ {
    tag: String,
    arguments: ElasticMQArgs,
}

impl Default for ElasticMQ {
    fn default() -> Self {
        ElasticMQ {
            tag: DEFAULT_TAG.to_string(),
            arguments: ElasticMQArgs {},
        }
    }
}

impl Image for ElasticMQ {
    type Args = ElasticMQArgs;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Started SQS rest server")
            .unwrap();
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        ElasticMQ { arguments, ..self }
    }
}

impl ElasticMQ {
    pub fn with_tag(self, tag_str: &str) -> Self {
        ElasticMQ {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
