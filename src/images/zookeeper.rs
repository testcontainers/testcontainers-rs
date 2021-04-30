use crate::{core::WaitFor, Image};

const CONTAINER_IDENTIFIER: &str = "zookeeper";
const DEFAULT_TAG: &str = "3.6.2";

#[derive(Debug, Default, Clone)]
pub struct ZookeeperArgs;
impl IntoIterator for ZookeeperArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct Zookeeper {
    tag: String,
    arguments: ZookeeperArgs,
}

impl Default for Zookeeper {
    fn default() -> Self {
        Zookeeper {
            tag: DEFAULT_TAG.to_string(),
            arguments: ZookeeperArgs {},
        }
    }
}
impl Image for Zookeeper {
    type Args = ZookeeperArgs;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started AdminServer")]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        Zookeeper { arguments, ..self }
    }
}
impl Zookeeper {
    pub fn with_tag(self, tag_str: &str) -> Self {
        Zookeeper {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
