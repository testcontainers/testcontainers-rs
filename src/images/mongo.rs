use crate::{core::WaitFor, Image};

const CONTAINER_IDENTIFIER: &str = "mongo";
const DEFAULT_TAG: &str = "4.0.17";

#[derive(Debug, Default, Clone)]
pub struct MongoArgs;

impl IntoIterator for MongoArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct Mongo {
    tag: String,
    arguments: MongoArgs,
}

impl Default for Mongo {
    fn default() -> Self {
        Mongo {
            tag: DEFAULT_TAG.to_string(),
            arguments: MongoArgs {},
        }
    }
}

impl Image for Mongo {
    type Args = MongoArgs;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "waiting for connections on port",
        )]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        Mongo { arguments, ..self }
    }
}

impl Mongo {
    pub fn with_tag(self, tag_str: &str) -> Self {
        Mongo {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
