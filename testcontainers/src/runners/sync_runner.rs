use crate::{core::error::Result, Container, Image, RunnableImage};

/// Helper trait to start containers synchronously.
///
/// ## Example
///
/// ```rust,no_run
/// use testcontainers::{core::WaitFor, runners::SyncRunner, GenericImage};
///
/// fn test_redis() {
///     let container = GenericImage::new("redis", "7.2.4")
///         .with_exposed_port(6379)
///         .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
///         .start()
///         .unwrap();
/// }
/// ```
pub trait SyncRunner<I: Image> {
    /// Starts the container and returns an instance of `Container`.
    fn start(self) -> Result<Container<I>>;

    /// Pulls the image from the registry.
    /// Useful if you want to pull the image before starting the container.
    fn pull_image(self) -> Result<RunnableImage<I>>;
}

impl<T, I> SyncRunner<I> for T
where
    T: Into<RunnableImage<I>> + Send,
    I: Image,
{
    fn start(self) -> Result<Container<I>> {
        let runtime = build_sync_runner()?;
        let async_container = runtime.block_on(super::AsyncRunner::start(self))?;

        Ok(Container::new(runtime, async_container))
    }

    fn pull_image(self) -> Result<RunnableImage<I>> {
        let runtime = build_sync_runner()?;
        runtime.block_on(super::AsyncRunner::pull_image(self))
    }
}

fn build_sync_runner() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, OnceLock},
    };

    use bollard_stubs::models::ContainerInspectResponse;
    use tokio::runtime::Runtime;

    use super::*;
    use crate::{
        core::{client::Client, mounts::Mount, WaitFor},
        images::generic::GenericImage,
        ImageExt,
    };

    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    fn runtime() -> &'static Runtime {
        RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
        })
    }

    fn docker_client() -> Arc<Client> {
        runtime().block_on(Client::lazy_client()).unwrap()
    }

    fn inspect(id: &str) -> ContainerInspectResponse {
        runtime().block_on(docker_client().inspect(id)).unwrap()
    }

    fn network_exists(client: &Arc<Client>, name: &str) -> bool {
        runtime().block_on(client.network_exists(name)).unwrap()
    }

    #[derive(Default)]
    struct HelloWorld {
        mounts: Vec<Mount>,
        env_vars: BTreeMap<String, String>,
    }

    impl Image for HelloWorld {
        fn name(&self) -> &str {
            "hello-world"
        }

        fn tag(&self) -> &str {
            "latest"
        }

        fn ready_conditions(&self) -> Vec<WaitFor> {
            vec![WaitFor::message_on_stdout("Hello from Docker!")]
        }

        fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
            Box::new(self.env_vars.iter())
        }

        fn mounts(&self) -> Box<dyn Iterator<Item = &Mount> + '_> {
            Box::new(self.mounts.iter())
        }
    }

    #[test]
    fn sync_run_command_should_expose_all_ports_if_no_explicit_mapping_requested(
    ) -> anyhow::Result<()> {
        let container = GenericImage::new("hello-world", "latest").start()?;

        let container_details = inspect(container.id());
        let publish_ports = container_details
            .host_config
            .expect("HostConfig")
            .publish_all_ports
            .expect("PublishAllPorts");
        assert!(publish_ports, "publish_all_ports must be `true`");
        Ok(())
    }

    #[test]
    fn sync_run_command_should_map_exposed_port() -> anyhow::Result<()> {
        let image = GenericImage::new("simple_web_server", "latest")
            .with_exposed_port(5000)
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));
        let container = image.start()?;
        let res = container.get_host_port_ipv4(5000);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn sync_run_command_should_expose_only_requested_ports() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image
            .with_mapped_port((124, 456))
            .with_mapped_port((556, 888))
            .start()?;

        let container_details = inspect(container.id());

        let port_bindings = container_details
            .host_config
            .expect("HostConfig")
            .port_bindings
            .expect("PortBindings");
        assert!(port_bindings.contains_key("456/tcp"));
        assert!(port_bindings.contains_key("888/tcp"));
        Ok(())
    }

    #[test]
    fn sync_rm_command_should_return_error_on_invalid_container() {
        let res = runtime().block_on(docker_client().rm("!INVALID_NAME_DUE_TO_SYMBOLS!"));
        assert!(
            res.is_err(),
            "should return an error on invalid container name"
        );
    }

    #[test]
    fn sync_run_command_should_include_network() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_network("sync-awesome-net-1").start()?;

        let container_details = inspect(container.id());
        let networks = container_details
            .network_settings
            .expect("NetworkSettings")
            .networks
            .expect("Networks");

        assert!(
            networks.contains_key("sync-awesome-net-1"),
            "Networks is {networks:?}"
        );
        Ok(())
    }

    #[test]
    fn sync_should_rely_on_network_mode_when_network_is_provided_and_settings_bridge_empty(
    ) -> anyhow::Result<()> {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = web_server.clone().with_network("bridge").start()?;

        assert!(!container.get_bridge_ip_address()?.to_string().is_empty());
        Ok(())
    }

    #[test]
    fn sync_should_return_error_when_non_bridged_network_selected() -> anyhow::Result<()> {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = web_server.clone().with_network("host").start()?;

        let res = container.get_bridge_ip_address();
        assert!(res.is_err());
        Ok(())
    }
    #[test]
    fn sync_run_command_should_include_name() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_container_name("sync_hello_container").start()?;

        let container_details = inspect(container.id());
        let container_name = container_details.name.expect("Name");
        assert!(container_name.ends_with("sync_hello_container"));
        Ok(())
    }

    #[test]
    fn sync_run_command_with_container_network_should_not_expose_ports() -> anyhow::Result<()> {
        let _first_container = GenericImage::new("simple_web_server", "latest")
            .with_container_name("the_first_one")
            .start()?;

        let image = GenericImage::new("hello-world", "latest");
        image.with_network("container:the_first_one").start()?;
        Ok(())
    }

    #[test]
    fn sync_run_command_should_include_privileged() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_privileged(true).start()?;
        let container_details = inspect(container.id());

        let privileged = container_details
            .host_config
            .expect("HostConfig")
            .privileged
            .expect("Privileged");
        assert!(privileged, "privileged must be `true`");
        Ok(())
    }

    #[test]
    fn sync_run_command_should_set_shared_memory_size() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_shm_size(1_000_000).start()?;

        let container_details = inspect(container.id());
        let shm_size = container_details
            .host_config
            .expect("HostConfig")
            .shm_size
            .expect("ShmSize");

        assert_eq!(shm_size, 1_000_000);
        Ok(())
    }

    #[test]
    fn sync_should_create_network_if_image_needs_it_and_drop_it_in_the_end() -> anyhow::Result<()> {
        {
            let client = docker_client();

            assert!(!network_exists(&client, "sync-awesome-net"));

            // creating the first container creates the network
            let _container1: Container<HelloWorld> = HelloWorld::default()
                .with_network("sync-awesome-net")
                .start()?;
            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = HelloWorld::default()
                .with_network("sync-awesome-net")
                .start()?;

            assert!(network_exists(&client, "sync-awesome-net"));
        }

        {
            let client = docker_client();
            // original client has been dropped, should clean up networks
            assert!(!network_exists(&client, "sync-awesome-net"));
        }
        Ok(())
    }
}
