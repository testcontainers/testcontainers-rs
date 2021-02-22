use crate::{
    core::{
        env, logs::LogStreamAsync, ports::Ports, ContainerAsync, DockerAsync, PortMapping, RunArgs,
    },
    Image,
};
use async_trait::async_trait;
use futures::{executor::block_on, stream::StreamExt, TryStreamExt};
use shiplift::{
    ContainerOptions, Docker, LogsOptions, NetworkCreateOptions, NetworkListOptions,
    RmContainerOptions,
};
use std::{
    fmt, io,
    sync::{Arc, RwLock},
};

/// A testcontainers client that uses HTTP to communicate with the docker daemon.
///
/// This client provides an async-based interface.
pub struct Http {
    inner: Arc<Client>,
}

/// The internal client.
///
/// This exists so we don't have to make the outer client clonable and still can have only a single instance around which is important for `Drop` behaviour.
struct Client {
    command: env::Command,
    shiplift: Docker,
    created_networks: RwLock<Vec<String>>,
}

impl fmt::Debug for Http {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Http").finish()
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

// public API
impl Http {
    pub async fn run<I: Image + Send + Sync>(&self, image: I) -> ContainerAsync<'_, I> {
        self.run_with_args(image, RunArgs::default()).await
    }

    pub async fn run_with_args<I: Image + Send + Sync>(
        &self,
        image: I,
        run_args: RunArgs,
    ) -> ContainerAsync<'_, I> {
        let mut options_builder = ContainerOptions::builder(image.descriptor().as_str());

        // Create network and add it to container creation
        if let Some(network) = run_args.network() {
            options_builder.network_mode(network.as_str());
            if self.create_network_if_not_exists(&network).await {
                let mut guard = self
                    .inner
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
        if let Some(ports) = run_args.ports() {
            for port in &ports {
                let (local, internal, protocol) = match port {
                    PortMapping::Tcp { local, internal } => (local, internal, "tcp"),
                    PortMapping::Udp { local, internal } => (local, internal, "udp"),
                    PortMapping::Sctp { local, internal } => (local, internal, "sctp"),
                };
                // casting u16 to u32
                options_builder.expose(*internal as u32, protocol, *local as u32);
            }
        } else {
            options_builder.publish_all_ports();
        }

        // create the container with options
        let create_result = self
            .inner
            .shiplift
            .containers()
            .create(&options_builder.build())
            .await;
        let id = create_result.unwrap().id;

        self.inner
            .shiplift
            .containers()
            .get(&id)
            .start()
            .await
            .unwrap();

        let client = Http {
            inner: self.inner.clone(),
        };

        ContainerAsync::new(id, client, image, self.inner.command).await
    }
}

impl Http {
    fn new() -> Self {
        Http {
            inner: Arc::new(Client {
                command: env::command::<env::Os>().unwrap_or_default(),
                shiplift: Docker::new(),
                created_networks: RwLock::new(Vec::new()),
            }),
        }
    }

    async fn create_network_if_not_exists(&self, network: &str) -> bool {
        if !network_exists(&self.inner.shiplift, network).await {
            self.inner
                .shiplift
                .networks()
                .create(&NetworkCreateOptions::builder(network).build())
                .await
                .unwrap();

            return true;
        }

        false
    }

    fn logs(&self, container_id: String, options: LogsOptions) -> LogStreamAsync<'_> {
        let stream = self
            .inner
            .shiplift
            .containers()
            .get(container_id)
            .logs(&options)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
            .map(|chunk| {
                let string = String::from_utf8(Vec::from(chunk?))
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                Ok(string)
            })
            .boxed();

        LogStreamAsync::new(stream)
    }
}

async fn network_exists(client: &Docker, network: &str) -> bool {
    // There's no public builder for NetworkListOptions yet
    // might need to add one in shiplift
    // this is doing unnecessary stuff
    let network_list_optons = NetworkListOptions::default();
    let networks = client.networks().list(&network_list_optons).await.unwrap();

    networks.iter().any(|i| i.name == network)
}

impl Drop for Client {
    fn drop(&mut self) {
        match self.command {
            env::Command::Remove => {
                let guard = self.created_networks.read().expect("failed to lock RwLock");
                for network in guard.iter() {
                    block_on(async {
                        self.shiplift
                            .networks()
                            .get(network)
                            .delete()
                            .await
                            .unwrap()
                    });
                }
            }
            env::Command::Keep => {}
        }
    }
}

#[async_trait]
impl DockerAsync for Http {
    fn stdout_logs<'s>(&'s self, id: &str) -> LogStreamAsync<'s> {
        self.logs(
            id.to_owned(),
            LogsOptions::builder().stdout(true).stderr(false).build(),
        )
    }

    fn stderr_logs<'s>(&'s self, id: &str) -> LogStreamAsync<'s> {
        self.logs(
            id.to_owned(),
            LogsOptions::builder().stdout(false).stderr(true).build(),
        )
    }

    async fn ports(&self, id: &str) -> Ports {
        let container_detatils = self
            .inner
            .shiplift
            .containers()
            .get(id)
            .inspect()
            .await
            .unwrap();

        container_detatils
            .network_settings
            .ports
            .map(Ports::new)
            .unwrap_or_default()
    }

    async fn rm(&self, id: &str) {
        self.inner
            .shiplift
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
        self.inner
            .shiplift
            .containers()
            .get(id)
            .stop(Option::None)
            .await
            .unwrap();
    }

    async fn start(&self, id: &str) {
        self.inner
            .shiplift
            .containers()
            .get(id)
            .start()
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::{generic::GenericImage, hello_world::HelloWorld};
    use shiplift::rep::ContainerDetails;
    use spectral::prelude::*;

    async fn inspect(client: &shiplift::Docker, id: &str) -> ContainerDetails {
        client.containers().get(id).inspect().await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let image = HelloWorld::default();
        let docker = Http::new();
        let container = docker.run(image).await;

        // inspect volume and env
        let container_details = inspect(&docker.inner.shiplift, container.id()).await;
        assert_that!(container_details.host_config.publish_all_ports).is_equal_to(true);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_only_requested_ports() {
        let image = GenericImage::new("hello-world");

        let docker = Http::new();
        let container = docker
            .run_with_args(
                image,
                RunArgs::default()
                    .with_mapped_port((123, 456))
                    .with_mapped_port((555, 888)),
            )
            .await;

        let container_details = inspect(&docker.inner.shiplift, container.id()).await;

        let port_bindings = container_details.host_config.port_bindings.unwrap();
        assert_that!(&port_bindings).contains_key(&"456/tcp".into());
        assert_that!(&port_bindings).contains_key(&"888/tcp".into());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_include_network() {
        let image = GenericImage::new("hello-world");
        let docker = Http::new();

        let run_args = RunArgs::default().with_network("awesome-net-1");
        let container = docker.run_with_args(image, run_args).await;

        let container_details = inspect(&docker.inner.shiplift, container.id()).await;
        let networks = container_details.network_settings.networks;

        assert!(
            networks.contains_key("awesome-net-1"),
            "Networks is {:?}",
            networks
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_include_name() {
        let image = GenericImage::new("hello-world");
        let docker = Http::new();

        let run_args = RunArgs::default().with_name("hello_container");
        let container = docker.run_with_args(image, run_args).await;

        let container_details = inspect(&docker.inner.shiplift, container.id()).await;
        assert_that!(container_details.name).ends_with("hello_container");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        let client = shiplift::Docker::new();

        {
            let docker = Http::new();
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
