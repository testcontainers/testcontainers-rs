use std::collections::HashMap;
use testcontainers::{core::WaitFor, Image};

const NAME: &str = "postgres";
const TAG: &str = "11-alpine";

#[derive(Debug)]
pub struct Postgres {
    env_vars: HashMap<String, String>,
}

impl Default for Postgres {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("POSTGRES_DB".to_owned(), "postgres".to_owned());
        env_vars.insert("POSTGRES_HOST_AUTH_METHOD".into(), "trust".into());

        Self { env_vars }
    }
}

impl Image for Postgres {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        )]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::postgres::Postgres as PostgresImage;
    use testcontainers::utils::free_local_port;
    use testcontainers::{clients, RunnableImage};

    #[test]
    fn postgres_one_plus_one() {
        let docker = clients::Cli::default();
        let postgres_image = PostgresImage::default();
        let node = docker.run(postgres_image);

        let connection_string = &format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
        );
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

        let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let first_row = &rows[0];
        let first_column: i32 = first_row.get(0);
        assert_eq!(first_column, 2);
    }

    #[test]
    fn postgres_one_plus_one_with_custom_mapped_port() {
        let _ = pretty_env_logger::try_init();
        let free_local_port = free_local_port().unwrap();

        let docker = clients::Cli::default();
        let image =
            RunnableImage::from(PostgresImage::default()).with_mapped_port((free_local_port, 5432));
        let _node = docker.run(image);

        let mut conn = postgres::Client::connect(
            &format!("postgres://postgres:postgres@localhost:{free_local_port}/postgres",),
            postgres::NoTls,
        )
        .unwrap();
        let rows = conn.query("SELECT 1+1 AS result;", &[]).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<_, i32>("result"), 2);
    }

    #[test]
    fn postgres_custom_version() {
        let docker = clients::Cli::default();
        let image = RunnableImage::from(PostgresImage::default()).with_tag("13-alpine");
        let node = docker.run(image);

        let connection_string = &format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
        );
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();

        let rows = conn.query("SELECT version()", &[]).unwrap();
        assert_eq!(rows.len(), 1);

        let first_row = &rows[0];
        let first_column: String = first_row.get(0);
        assert!(first_column.contains("13"));
    }
}
