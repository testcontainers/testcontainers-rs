use tc_core::{Container, Docker, Image, WaitForMessage};

#[derive(Debug)]
pub struct Postgres {
    arguments: PostgresArgs,
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
        Self {
            arguments: PostgresArgs::default(),
        }
    }
}

impl Image for Postgres {
    type Args = PostgresArgs;

    fn descriptor(&self) -> String {
        "postgres:11-alpine".to_string()
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        container
            .logs()
            .stderr
            .wait_for_message("database system is ready to accept connections")
            .unwrap();
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }
}
