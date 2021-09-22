use crate::{core::WaitFor, Image};

const NAME: &str = "zookeeper";
const TAG: &str = "3.6.2";

#[derive(Debug, Default)]
pub struct Zookeeper;

impl Image for Zookeeper {
    type Args = ();

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
