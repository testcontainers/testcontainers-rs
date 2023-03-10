use std::collections::HashMap;
use testcontainers::{core::WaitFor, Image};

const NAME: &str = "orientdb";
const TAG: &str = "3.1.3";

#[derive(Debug)]
pub struct OrientDb {
    env_vars: HashMap<String, String>,
}

impl Default for OrientDb {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ORIENTDB_ROOT_PASSWORD".to_owned(), "root".to_owned());

        OrientDb { env_vars }
    }
}

impl Image for OrientDb {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("OrientDB Studio available at")]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::orientdb::OrientDb;
    use testcontainers::clients;

    #[test]
    #[ignore]
    fn orientdb_exists_database() {
        let docker = clients::Cli::default();
        let orientdb_image = OrientDb::default();
        let node = docker.run(orientdb_image);

        let client =
            orientdb_client::OrientDB::connect(("127.0.0.1", node.get_host_port_ipv4(2424)))
                .unwrap();

        let exists = client
            .exist_database(
                "orientdb_exists_database",
                "root",
                "root",
                orientdb_client::DatabaseType::Memory,
            )
            .unwrap();

        assert!(!exists);
    }
}
