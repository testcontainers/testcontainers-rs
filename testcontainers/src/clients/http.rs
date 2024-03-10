use crate::{
    core::{env, logs::LogStreamAsync, ports::Ports, DockerAsync, Port},
    ContainerAsync, Image, ImageArgs, RunnableImage,
};
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions},
    image::CreateImageOptions,
    models::{ContainerCreateResponse, ContainerInspectResponse, HostConfig, PortBinding},
    network::CreateNetworkOptions,
    Docker,
};
use futures::{executor::block_on, stream::StreamExt, TryStreamExt};
use std::{
    collections::HashMap,
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
    bollard: Docker,
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
    pub async fn run<I: Image>(&self, image: impl Into<RunnableImage<I>>) -> ContainerAsync<I> {
        let image = image.into();
        let mut create_options: Option<CreateContainerOptions<String>> = None;
        let mut config: Config<String> = Config {
            image: Some(image.descriptor()),
            host_config: Some(HostConfig::default()),
            ..Default::default()
        };

        // shared memory
        if let Some(bytes) = image.shm_size() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.shm_size = Some(bytes as i64);
                host_config
            });
        }

        // create network and add it to container creation
        if let Some(network) = image.network() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.network_mode = Some(network.to_string());
                host_config
            });
            if self.create_network_if_not_exists(network).await {
                let mut guard = self
                    .inner
                    .created_networks
                    .write()
                    .expect("'failed to lock RwLock'");
                guard.push(network.clone());
            }
        }

        // name of the container
        if let Some(name) = image.container_name() {
            create_options = Some(CreateContainerOptions {
                name: name.to_owned(),
            })
        }

        // handle environment variables
        let envs: Vec<String> = image
            .env_vars()
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        config.env = Some(envs);

        // volumes
        let vols: HashMap<String, HashMap<(), ()>> = image
            .volumes()
            .into_iter()
            .map(|(orig, dest)| (format!("{orig}:{dest}"), HashMap::new()))
            .collect();
        config.volumes = Some(vols);

        // entrypoint
        if let Some(entrypoint) = image.entrypoint() {
            config.entrypoint = Some(vec![entrypoint]);
        }

        // exposed ports
        config.exposed_ports = Some(
            image
                .expose_ports()
                .into_iter()
                .map(|p| (format!("{p}/tcp"), HashMap::new()))
                .collect(),
        );

        // ports
        if image.ports().is_some() || image.expose_ports().len() > 0 {
            let empty: Vec<Port> = Vec::new();
            let bindings = image
                .ports()
                .as_ref()
                .unwrap_or(&empty)
                .iter()
                .map(|p| {
                    (
                        format!("{}/tcp", p.internal),
                        Some(vec![PortBinding {
                            host_ip: Some(String::from("127.0.0.1")),
                            host_port: Some(p.local.to_string()),
                        }]),
                    )
                })
                .chain(
                    image
                        .expose_ports()
                        .into_iter()
                        .map(|p| (format!("{}/tcp", p), Some(vec![PortBinding::default()]))),
                );

            config.host_config = config.host_config.map(|mut host_config| {
                host_config.port_bindings = Some(bindings.collect());
                host_config
            });
        } else {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.publish_all_ports = Some(true);
                host_config
            });
        }

        let args = image
            .args()
            .clone()
            .into_iterator()
            .collect::<Vec<String>>();
        if !args.is_empty() {
            config.cmd = Some(args);
        }

        // create the container with options
        let create_result = self
            .create_container(create_options.clone(), config.clone())
            .await;
        let container_id = {
            match create_result {
                Ok(container) => container.id,
                Err(bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                }) => {
                    {
                        let pull_options = Some(CreateImageOptions {
                            from_image: image.descriptor(),
                            ..Default::default()
                        });
                        let mut pulling = self.inner.bollard.create_image(pull_options, None, None);
                        while let Some(result) = pulling.next().await {
                            if result.is_err() {
                                result.unwrap();
                            }
                        }
                    }
                    self.create_container(create_options, config)
                        .await
                        .unwrap()
                        .id
                }
                Err(err) => panic!("{}", err),
            }
        };

        #[cfg(feature = "watchdog")]
        if self.inner.command == env::Command::Remove {
            crate::watchdog::register(container_id.clone());
        }

        self.inner
            .bollard
            .start_container::<String>(&container_id, None)
            .await
            .unwrap();

        let client = Http {
            inner: self.inner.clone(),
        };

        ContainerAsync::new(container_id, client, image, self.inner.command).await
    }
}

impl Http {
    fn new() -> Self {
        Http {
            inner: Arc::new(Client {
                command: env::command::<env::Os>().unwrap_or_default(),
                bollard: Docker::connect_with_http_defaults().unwrap(),
                created_networks: RwLock::new(Vec::new()),
            }),
        }
    }

    async fn create_network_if_not_exists(&self, network: &str) -> bool {
        if !network_exists(&self.inner.bollard, network).await {
            self.inner
                .bollard
                .create_network(CreateNetworkOptions {
                    name: network.to_owned(),
                    ..Default::default()
                })
                .await
                .unwrap();

            return true;
        }

        false
    }

    async fn create_container(
        &self,
        options: Option<CreateContainerOptions<String>>,
        config: Config<String>,
    ) -> Result<ContainerCreateResponse, bollard::errors::Error> {
        self.inner.bollard.create_container(options, config).await
    }

    fn logs(&self, container_id: String, options: LogsOptions<String>) -> LogStreamAsync<'_> {
        let stream = self
            .inner
            .bollard
            .logs(&container_id, Some(options))
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
            .map(|chunk| {
                let bytes = chunk?.into_bytes();
                let str = std::str::from_utf8(bytes.as_ref())
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                Ok(String::from(str))
            })
            .boxed();

        LogStreamAsync::new(stream)
    }
}

async fn network_exists(client: &Docker, network: &str) -> bool {
    let networks = client.list_networks::<String>(None).await.unwrap();
    networks
        .iter()
        .any(|i| matches!(&i.name, Some(name) if name == network))
}

impl Drop for Client {
    fn drop(&mut self) {
        match self.command {
            env::Command::Remove => {
                let guard = self.created_networks.read().expect("failed to lock RwLock");
                for network in guard.iter() {
                    block_on(async { self.bollard.remove_network(network).await.unwrap() });
                }
            }
            env::Command::Keep => {}
        }
    }
}

#[async_trait]
impl DockerAsync for Http {
    fn stdout_logs(&self, id: &str) -> LogStreamAsync<'_> {
        self.logs(
            id.to_owned(),
            LogsOptions {
                follow: true,
                stdout: true,
                tail: "all".to_owned(),
                ..Default::default()
            },
        )
    }

    fn stderr_logs(&self, id: &str) -> LogStreamAsync<'_> {
        self.logs(
            id.to_owned(),
            LogsOptions {
                follow: true,
                stderr: true,
                tail: "all".to_owned(),
                ..Default::default()
            },
        )
    }

    async fn ports(&self, id: &str) -> Ports {
        self.inspect(id)
            .await
            .network_settings
            .unwrap_or_default()
            .ports
            .map(Ports::from)
            .unwrap_or_default()
    }

    async fn inspect(&self, id: &str) -> ContainerInspectResponse {
        self.inner
            .bollard
            .inspect_container(id, None)
            .await
            .unwrap()
    }

    async fn rm(&self, id: &str) {
        self.inner
            .bollard
            .remove_container(
                id,
                Some(RemoveContainerOptions {
                    force: true,
                    v: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
    }

    async fn stop(&self, id: &str) {
        self.inner.bollard.stop_container(id, None).await.unwrap();
    }

    async fn start(&self, id: &str) {
        self.inner
            .bollard
            .start_container::<String>(id, None)
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::generic::GenericImage;

    async fn inspect(client: &bollard::Docker, id: &str) -> ContainerInspectResponse {
        client.inspect_container(id, None).await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let docker = Http::new();
        let image = GenericImage::new("hello-world", "latest");
        let container = docker.run(image).await;

        // inspect volume and env
        let container_details = inspect(&docker.inner.bollard, container.id()).await;
        let publish_ports = container_details
            .host_config
            .unwrap()
            .publish_all_ports
            .unwrap();
        assert_eq!(publish_ports, true, "publish_all_ports must be `true`");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_map_exposed_port() {
        let docker = Http::new();
        let image = GenericImage::new("simple_web_server", "latest").with_exposed_port(5000);
        let container = docker.run(image).await;
        container.get_host_port_ipv4(5000).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_only_requested_ports() {
        let docker = Http::new();
        let image = GenericImage::new("hello-world", "latest");
        let image = RunnableImage::from(image)
            .with_mapped_port((123, 456))
            .with_mapped_port((555, 888));
        let container = docker.run(image).await;

        let container_details = inspect(&docker.inner.bollard, container.id()).await;

        let port_bindings = container_details
            .host_config
            .unwrap()
            .port_bindings
            .unwrap();
        assert!(port_bindings.contains_key("456/tcp"));
        assert!(port_bindings.contains_key("888/tcp"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_include_network() {
        let docker = Http::new();
        let image = GenericImage::new("hello-world", "latest");
        let image = RunnableImage::from(image).with_network("awesome-net-1");
        let container = docker.run(image).await;

        let container_details = inspect(&docker.inner.bollard, container.id()).await;
        let networks = container_details
            .network_settings
            .unwrap()
            .networks
            .unwrap();

        assert!(
            networks.contains_key("awesome-net-1"),
            "Networks is {networks:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_include_name() {
        let docker = Http::new();
        let image = GenericImage::new("hello-world", "latest");
        let image = RunnableImage::from(image).with_container_name("hello_container");
        let container = docker.run(image).await;

        let container_details = inspect(&docker.inner.bollard, container.id()).await;
        let container_name = container_details.name.unwrap();
        assert!(container_name.ends_with("hello_container"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        let client = bollard::Docker::connect_with_http_defaults().unwrap();
        let hello_world = GenericImage::new("hello-world", "latest");

        {
            let docker = Http::new();
            assert!(!network_exists(&client, "awesome-net-2").await);

            // creating the first container creates the network
            let _container1 = docker
                .run(RunnableImage::from(hello_world.clone()).with_network("awesome-net-2"))
                .await;

            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = docker
                .run(RunnableImage::from(hello_world).with_network("awesome-net-2"))
                .await;

            assert!(network_exists(&client, "awesome-net-2").await);
        }

        // client has been dropped, should clean up networks
        assert!(!network_exists(&client, "awesome-net-2").await)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_set_shared_memory_size() {
        let docker = Http::new();
        let image = GenericImage::new("hello-world", "latest");
        let image = RunnableImage::from(image).with_shm_size(1_000_000);
        let container = docker.run(image).await;

        let container_details = inspect(&docker.inner.bollard, container.id()).await;
        let shm_size = container_details.host_config.unwrap().shm_size.unwrap();

        assert_eq!(shm_size, 1_000_000);
    }
}
