use testcontainers::{core::WaitFor, Image};

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

#[cfg(test)]
mod tests {
    use crate::redis::Redis;
    use redis::Commands;
    use testcontainers::clients;

    #[test]
    fn redis_fetch_an_integer() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(Redis::default());
        let host_port = node.get_host_port_ipv4(6379);
        let url = format!("redis://127.0.0.1:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        con.set::<_, _, ()>("my_key", 42).unwrap();
        let result: i64 = con.get("my_key").unwrap();
        assert_eq!(42, result);
    }
}
