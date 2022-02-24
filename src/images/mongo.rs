use crate::{core::WaitFor, Image};

const NAME: &str = "mongo";
const TAG: &str = "5.0.6";

#[derive(Default, Debug)]
pub struct Mongo;

impl Image for Mongo {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Waiting for connections")]
    }
}
