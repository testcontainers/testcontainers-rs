use crate::{core::WaitFor, Image};

const NAME: &str = "zookeeper";
const TAG: &str = "3.6.2";

#[derive(Debug, Default, Clone)]
pub struct ZookeeperArgs;

impl IntoIterator for ZookeeperArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug, Default)]
pub struct Zookeeper;

impl Image for Zookeeper {
    type Args = ZookeeperArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started AdminServer")]
    }
}
