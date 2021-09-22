use crate::{core::WaitFor, Image};

const NAME: &str = "redis";
const TAG: &str = "5.0";

#[derive(Debug, Default)]
pub struct Redis;

impl Image for Redis {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }
}
