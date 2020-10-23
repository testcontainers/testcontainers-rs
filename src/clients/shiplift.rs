use crate::core::{ContainerAsync, DockerAsync, ImageAsync, LogsAsync, Ports, RunArgs};
use async_trait::async_trait;
use futures::executor::block_on;
use futures::stream::StreamExt;
use shiplift::Docker;
use shiplift::{
    ContainerOptions, LogsOptions, NetworkCreateOptions, NetworkListOptions, RmContainerOptions,
};
use std::fmt;
use std::sync::RwLock;

pub struct Shiplift {
    client: Docker,
    created_networks: RwLock<Vec<String>>,
}

impl fmt::Debug for Shiplift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shiplift").finish()
    }
}

impl Shiplift {
    pub fn new() -> Self {
        return Shiplift {
            client: Docker::new(),
            created_networks: RwLock::new(Vec::new()),
        };
    }

    async fn create_network_if_not_exists(&self, network: &String) -> bool {
        if !network_exists(&self.client, network).await {
            self.client
                .networks()
                .create(&NetworkCreateOptions::builder(network.as_str()).build())
                .await
                .unwrap();

            return true;
        }

        false
    }
}

async fn network_exists(client: &Docker, network: &str) -> bool {
    // There's no public builder for NetworkListOptions yet
    // might need to add one in shiplift
    // this is doing unnecessary stuff
    let network_list_optons = NetworkListOptions::default();
    let networks = client.networks().list(&network_list_optons).await.unwrap();
    println!("{:?}", networks);

    networks.iter().any(|i| i.name == network)
}

impl Drop for Shiplift {
    fn drop(&mut self) {
        let guard = self.created_networks.read().expect("failed to lock RwLock");
        for network in guard.iter() {
            block_on(async { self.client.networks().get(network).delete().await.unwrap() });
        }
    }
}

#[async_trait]
impl DockerAsync for Shiplift {
    async fn run<I: ImageAsync + Sync>(&self, image: I) -> ContainerAsync<'_, Shiplift, I> {
        let empty_args = RunArgs::default();
        self.run_with_args(image, empty_args).await
    }

    async fn run_with_args<I: ImageAsync + Send + Sync>(
        &self,
        image: I,
        run_args: RunArgs,
    ) -> ContainerAsync<'_, Shiplift, I> {
        let mut options_builder = ContainerOptions::builder(image.descriptor().as_str());

        // Create network and add it to container creation
        if let Some(network) = run_args.network() {
            options_builder.network_mode(network.as_str());
            if self.create_network_if_not_exists(&network).await {
                let mut guard = self
                    .created_networks
                    .write()
                    .expect("'failed to lock RwLock'");
                guard.push(network);
            }
        }

        // name of the container
        if let Some(name) = run_args.name() {
            options_builder.name(name.as_str());
        }

        // handle environment variables
        let envs: Vec<String> = image
            .env_vars()
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // the fact .env and .volumes takes Vec<&str> instead of AsRef<str> is making
        // this more difficult than it needs to be
        let envs_str: Vec<&str> = envs.iter().map(|s| s.as_ref()).collect();
        options_builder.env(envs_str);

        // volumes
        let vols: Vec<String> = image
            .volumes()
            .into_iter()
            .map(|(orig, dest)| format!("{}:{}", orig, dest))
            .collect();
        let vols_str: Vec<&str> = vols.iter().map(|s| s.as_ref()).collect();
        options_builder.volumes(vols_str);

        // entrypoint
        if let Some(entrypoint) = image.entrypoint() {
            options_builder.entrypoint(entrypoint.as_str());
        }

        // ports
        if let Some(ports) = image.ports() {
            // TODO support UDP?
            for port in &ports {
                // casting u16 to u32
                options_builder.expose(port.internal as u32, "tcp", port.local as u32);
            }
        } else {
            options_builder.publish_all_ports();
        }

        // create the container with options
        let create_result = self
            .client
            .containers()
            .create(&options_builder.build())
            .await;
        let id = create_result.unwrap().id;

        self.client.containers().get(&id).start().await.unwrap();

        ContainerAsync::new(id, self, image).await
    }

    // static str here might not be the best option
    async fn logs<'a>(&'a self, id: &'a str) -> LogsAsync<'a> {
        // XXX Need advice to handle these two unwrap
        let logs_stream_stdout = self
            .client
            .containers()
            .get(id)
            .logs(&LogsOptions::builder().stdout(true).stderr(false).build())
            .map(|chunk| chunk.unwrap().into());

        let logs_stream_stderr = self
            .client
            .containers()
            .get(id)
            .logs(&LogsOptions::builder().stdout(false).stderr(true).build())
            .map(|chunk| chunk.unwrap().into());

        LogsAsync {
            stdout: Box::new(logs_stream_stdout),
            stderr: Box::new(logs_stream_stderr),
        }
    }

    async fn ports(&self, id: &str) -> Ports {
        let mut ports = Ports::default();
        let container_detatils = self.client.containers().get(id).inspect().await.unwrap();

        //TODO should implement into_ports on external API port
        if let Some(inspect_ports) = container_detatils.network_settings.ports {
            for (internal, external) in inspect_ports {
                // PortMapping here is actualy a HashMap
                // NetworkSettings -> Port -> Vec<HashMap<String, String>>
                // therefore pop -> first key using next even though it's a map
                let external = match external
                    .and_then(|mut m| m.pop())
                    //XXX this is bad, need advice...
                    .map(|m| m.values().next().unwrap().clone())
                {
                    Some(port) => port,
                    None => {
                        log::debug!("Port {} is not mapped to host machine, skipping.", internal);
                        continue;
                    }
                };

                let port = internal.split('/').next().unwrap();

                let internal = parse_port(port);
                let external = parse_port(&external);

                ports.add_mapping(internal, external);
            }
        }

        ports
    }

    async fn rm(&self, id: &str) {
        self.client
            .containers()
            .get(id)
            .remove(
                RmContainerOptions::builder()
                    .volumes(true)
                    .force(true)
                    .build(),
            )
            .await
            .unwrap();
    }

    async fn stop(&self, id: &str) {
        self.client
            .containers()
            .get(id)
            .stop(Option::None)
            .await
            .unwrap();
    }

    async fn start(&self, id: &str) {
        self.client.containers().get(id).start().await.unwrap();
    }
}

fn parse_port(port: &str) -> u16 {
    port.parse()
        .unwrap_or_else(|e| panic!("Failed to parse {} as u16 because {}", port, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Port;
    use crate::images::generic_async::GenericImageAsync;
    use crate::{core::ContainerAsync, core::DockerAsync, core::ImageAsync};
    use shiplift::rep::ContainerDetails;
    use spectral::prelude::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct HelloWorld {
        volumes: HashMap<String, String>,
        env_vars: HashMap<String, String>,
    }

    #[async_trait]
    impl ImageAsync for HelloWorld {
        type Args = Vec<String>;
        type EnvVars = HashMap<String, String>;
        type Volumes = HashMap<String, String>;
        type EntryPoint = std::convert::Infallible;

        fn descriptor(&self) -> String {
            String::from("hello-world")
        }

        async fn wait_until_ready<D: DockerAsync + Sync>(
            &self,
            _container: &ContainerAsync<'_, D, Self>,
        ) {
        }

        fn args(&self) -> <Self as ImageAsync>::Args {
            vec![]
        }

        fn volumes(&self) -> Self::Volumes {
            self.volumes.clone()
        }

        fn env_vars(&self) -> Self::EnvVars {
            self.env_vars.clone()
        }

        fn ports(&self) -> Option<Vec<Port>> {
            None
        }

        fn with_args(self, _arguments: <Self as ImageAsync>::Args) -> Self {
            self
        }
    }

    // A simple test to make sure basic functionality works
    // complete functional test suite in tests/shiplift_client.rs
    #[tokio::test(threaded_scheduler)]
    async fn shiplift_can_run_container() {
        let image = HelloWorld::default();
        let shiplift = Shiplift::new();
        shiplift.run(image).await;
    }

    async fn inspect(client: &shiplift::Docker, id: &str) -> ContainerDetails {
        client.containers().get(id).inspect().await.unwrap()
    }

    #[tokio::test(threaded_scheduler)]
    async fn shiplift_run_command_should_include_env_and_volumes() {
        let mut volumes = HashMap::new();
        volumes.insert("/tmp".to_owned(), "/hostmp".to_owned());

        let mut env_vars = HashMap::new();
        env_vars.insert("one-key".to_owned(), "one-value".to_owned());
        env_vars.insert("two-key".to_owned(), "two-value".to_owned());

        let image = HelloWorld { volumes, env_vars };
        let run_args = RunArgs::default();

        let docker = Shiplift::new();
        let container = docker.run_with_args(image, run_args).await;

        // inspect volume and env
        let container_details = inspect(&docker.client, container.id()).await;

        let envs = container_details.config.env.unwrap();
        assert_that!(&envs).contains(&"one-key=one-value".into());
        assert_that!(&envs).contains(&"two-key=two-value".into());
    }

    #[tokio::test(threaded_scheduler)]
    async fn shiplift_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let image = HelloWorld::default();
        let docker = Shiplift::new();
        let container = docker.run(image).await;

        // inspect volume and env
        let container_details = inspect(&docker.client, container.id()).await;
        assert_that!(container_details.host_config.publish_all_ports).is_equal_to(true);
    }

    #[tokio::test(threaded_scheduler)]
    async fn shiplift_run_command_should_expose_only_requested_ports() {
        let image = GenericImageAsync::new("hello-world")
            .with_mapped_port((123, 456))
            .with_mapped_port((555, 888));

        let docker = Shiplift::new();
        let container = docker.run(image).await;

        let container_details = inspect(&docker.client, container.id()).await;

        let port_bindings = container_details.host_config.port_bindings.unwrap();
        assert_that!(&port_bindings).contains_key(&"456/tcp".into());
        assert_that!(&port_bindings).contains_key(&"888/tcp".into());
    }

    #[tokio::test(threaded_scheduler)]
    async fn shiplift_run_command_should_include_network() {
        let image = GenericImageAsync::new("hello-world");
        let docker = Shiplift::new();

        let run_args = RunArgs::default().with_network("awesome-net-1");
        let container = docker.run_with_args(image, run_args).await;

        let container_details = inspect(&docker.client, container.id()).await;
        let networks = container_details.network_settings.networks;
        assert!(
            networks.contains_key("awesome-net-1".into()),
            format!("Networks is {:?}", networks)
        );
    }

    #[tokio::test(threaded_scheduler)]
    async fn shiplift_run_command_should_include_name() {
        let image = GenericImageAsync::new("hello-world");
        let docker = Shiplift::new();

        let run_args = RunArgs::default().with_name("hello_container");
        let container = docker.run_with_args(image, run_args).await;

        let container_details = inspect(&docker.client, container.id()).await;
        assert_that!(container_details.name).ends_with("hello_container");
    }

    #[tokio::test(threaded_scheduler)]
    async fn shiplift_should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        let client = shiplift::Docker::new();

        {
            let docker = Shiplift::new();
            assert!(!network_exists(&client, "awesome-net-2").await);

            // creating the first container creates the network
            let _container1 = docker
                .run_with_args(
                    HelloWorld::default(),
                    RunArgs::default().with_network("awesome-net-2"),
                )
                .await;
            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = docker
                .run_with_args(
                    HelloWorld::default(),
                    RunArgs::default().with_network("awesome-net-2"),
                )
                .await;

            assert!(network_exists(&client, "awesome-net-2").await);
        }

        // client has been dropped, should clean up networks
        assert!(!network_exists(&client, "awesome-net-2").await)
    }
}
