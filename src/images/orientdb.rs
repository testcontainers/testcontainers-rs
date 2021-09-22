use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const NAME: &str = "orientdb";
const DEFAULT_TAG: &str = "3.1.3";

#[derive(Debug, Default, Clone)]
pub struct OrientDbArgs;

impl IntoIterator for OrientDbArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct OrientDb {
    tag: String,
    arguments: OrientDbArgs,
    env_vars: HashMap<String, String>,
}

impl Default for OrientDb {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ORIENTDB_ROOT_PASSWORD".to_owned(), "root".to_owned());

        OrientDb {
            tag: DEFAULT_TAG.to_string(),
            arguments: OrientDbArgs {},
            env_vars,
        }
    }
}

impl Image for OrientDb {
    type Args = OrientDbArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("OrientDB Studio available at")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}
