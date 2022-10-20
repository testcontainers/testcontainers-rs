use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const NAME: &str = "mysql";
const TAG: &str = "8.0";

#[derive(Debug)]
pub struct MySql {
    env_vars: HashMap<String, String>,
}

impl Default for MySql {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("MYSQL_ROOT_PASSWORD".to_owned(), "password".to_owned());
        env_vars.insert("MYSQL_DATABASE".into(), "test".into());

        Self { env_vars }
    }
}

impl Image for MySql {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("ready for connections"),
            WaitFor::message_on_stderr("port: 3306"),
        ]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}
