use testcontainers::{core::WaitFor, Image};

const NAME: &str = "zookeeper";
const TAG: &str = "3.6.2";

#[derive(Debug, Default)]
pub struct Zookeeper;

impl Image for Zookeeper {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Started AdminServer")]
    }
}

#[cfg(test)]
mod tests {
    use crate::zookeeper::Zookeeper as ZookeeperImage;
    use std::time::Duration;
    use testcontainers::clients;
    use zookeeper::Acl;
    use zookeeper::CreateMode;
    use zookeeper::ZooKeeper;

    #[test]
    #[ignore]
    fn zookeeper_check_directories_existence() {
        let _ = pretty_env_logger::try_init();

        let docker = clients::Cli::default();
        let image = ZookeeperImage::default();
        let node = docker.run(image);

        let host_port = node.get_host_port_ipv4(2181);
        let zk_urls = format!("127.0.0.1:{host_port}");
        let zk = ZooKeeper::connect(&zk_urls, Duration::from_secs(15), |_| ()).unwrap();

        zk.create(
            "/test",
            vec![1, 2],
            Acl::open_unsafe().clone(),
            CreateMode::Ephemeral,
        )
        .unwrap();

        assert!(matches!(zk.exists("/test", false).unwrap(), Some(_)));
        assert!(matches!(zk.exists("/test2", false).unwrap(), None));
    }
}
