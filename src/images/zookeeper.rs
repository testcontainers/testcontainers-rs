use crate::{core::WaitFor, Image};

const NAME: &str = "zookeeper";
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

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started AdminServer")]
    }
}
