use crate::{core::WaitFor, Image};

const NAME: &str = "mongo";
const TAG: &str = "4.0.17";

#[derive(Debug, Default, Clone)]
pub struct MongoArgs;

impl IntoIterator for MongoArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Default, Debug)]
pub struct Mongo;

impl Image for Mongo {
    type Args = MongoArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "waiting for connections on port",
        )]
    }
}
