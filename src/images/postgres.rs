use crate::{core::WaitFor, Image};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Postgres {
    arguments: PostgresArgs,
    env_vars: HashMap<String, String>,
    version: u8,
}

#[derive(Default, Debug, Clone)]
pub struct PostgresArgs {}

impl IntoIterator for PostgresArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        vec![].into_iter()
    }
}

impl Default for Postgres {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("POSTGRES_DB".to_owned(), "postgres".to_owned());
        env_vars.insert("POSTGRES_HOST_AUTH_METHOD".into(), "trust".into());

        Self {
            arguments: PostgresArgs::default(),
            env_vars,
            version: 11,
        }
    }
}
impl Postgres {
    pub fn with_env_vars(self, env_vars: HashMap<String, String>) -> Self {
        Self { env_vars, ..self }
    }

    pub fn with_version(self, version: u8) -> Self {
        Self { version, ..self }
    }
}

impl Image for Postgres {
    type Args = PostgresArgs;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("postgres:{}-alpine", self.version)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        )]
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }
}
