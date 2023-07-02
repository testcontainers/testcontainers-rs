use crate::{
    core::{env, logs::LogStreamAsync, ports::Ports, DockerAsync},
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
    io,
    sync::{OnceLock, RwLock},
};

static HTTP_DOCKER: OnceLock<Client> = OnceLock::new();

fn docker_client() -> &'static Client {
    HTTP_DOCKER.get_or_init(|| Client::new())
}

/// The internal client.
///
/// This exists so we don't have to make the outer client clonable and still can have only a single instance around which is important for `Drop` behaviour.
struct Client {
    command: env::Command,
    bollard: Docker,
    created_networks: RwLock<Vec<String>>,
}

#[async_trait]
pub trait RunViaHttp<I: Image> {
    async fn start(self) -> ContainerAsync<I>;
}

#[async_trait]
impl<I> RunViaHttp<I> for I
where
    I: Image,
    I::Args: Default,
{
    async fn start(self) -> ContainerAsync<I> {
        RunnableImage::from(self).start().await
    }
}

#[async_trait]
impl<I: Image> RunViaHttp<I> for RunnableImage<I> {
    async fn start(self) -> ContainerAsync<I> {
        let client = docker_client();
        let mut create_options: Option<CreateContainerOptions<String>> = None;
        let mut config: Config<String> = Config {
            image: Some(self.descriptor()),
            host_config: Some(HostConfig::default()),
            ..Default::default()
        };

        // shared memory
        if let Some(bytes) = self.shm_size() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.shm_size = Some(bytes as i64);
                host_config
            });
        }

        // create network and add it to container creation
        if let Some(network) = self.network() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.network_mode = Some(network.to_string());
                host_config
            });
            if client.create_network_if_not_exists(network).await {
                let mut guard = client
                    .created_networks
                    .write()
                    .expect("'failed to lock RwLock'");
                guard.push(network.clone());
            }
        }

        // name of the container
        if let Some(name) = self.container_name() {
            create_options = Some(CreateContainerOptions {
                name: name.to_owned(),
            })
        }

        // handle environment variables
        let envs: Vec<String> = self
            .env_vars()
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        config.env = Some(envs);

        // volumes
        let vols: HashMap<String, HashMap<(), ()>> = self
            .volumes()
            .into_iter()
            .map(|(orig, dest)| (format!("{orig}:{dest}"), HashMap::new()))
            .collect();
        config.volumes = Some(vols);

        // entrypoint
        if let Some(entrypoint) = self.entrypoint() {
            config.entrypoint = Some(vec![entrypoint]);
        }

        // ports
        if let Some(ports) = self.ports() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.port_bindings = Some(HashMap::new());
                host_config
            });

            for port in ports {
                config.host_config = config.host_config.map(|mut host_config| {
                    host_config.port_bindings =
                        host_config.port_bindings.map(|mut port_bindings| {
                            port_bindings.insert(
                                format!("{}/tcp", port.internal),
                                Some(vec![PortBinding {
                                    host_ip: Some(String::from("127.0.0.1")),
                                    host_port: Some(port.local.to_string()),
                                }]),
                            );

                            port_bindings
                        });

                    host_config
                });
            }
        } else {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.publish_all_ports = Some(true);
                host_config
            });
        }

        let args = self.args().clone().into_iterator().collect::<Vec<String>>();
        if !args.is_empty() {
            config.cmd = Some(args);
        }

        // create the container with options
        let create_result = client
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
                            from_image: self.descriptor(),
                            ..Default::default()
                        });
                        let mut pulling = client.bollard.create_image(pull_options, None, None);
                        while let Some(result) = pulling.next().await {
                            if result.is_err() {
                                result.unwrap();
                            }
                        }
                    }
                    client
                        .bollard
                        .create_container(create_options, config)
                        .await
                        .unwrap()
                        .id
                }
                Err(err) => panic!("{}", err),
            }
        };

        #[cfg(feature = "watchdog")]
        if client.command == env::Command::Remove {
            crate::watchdog::register(container_id.clone());
        }

        client
            .bollard
            .start_container::<String>(&container_id, None)
            .await
            .unwrap();

        ContainerAsync::new(container_id, client.clone(), self, client.command).await
    }
}

impl Client {
    fn new() -> Client {
        Client {
            command: env::command::<env::Os>().unwrap_or_default(),
            bollard: Docker::connect_with_unix_defaults()
                .expect("Failed to initialize docker client"),
            created_networks: RwLock::new(Vec::new()),
        }
    }

    async fn create_network_if_not_exists(&self, network: &str) -> bool {
        if !network_exists(&self.bollard, network).await {
            self.bollard
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
        self.bollard.create_container(options, config).await
    }

    fn logs(&self, container_id: String, options: LogsOptions<String>) -> LogStreamAsync<'_> {
        let stream = self
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
impl DockerAsync for &Client {
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
        self.bollard.inspect_container(id, None).await.unwrap()
    }

    async fn rm(&self, id: &str) {
        self.bollard
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
        self.bollard.stop_container(id, None).await.unwrap();
    }

    async fn start(&self, id: &str) {
        self.bollard
            .start_container::<String>(id, None)
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::generic::GenericImage;
    use spectral::prelude::*;

    async fn inspect(client: &bollard::Docker, id: &str) -> ContainerInspectResponse {
        client.inspect_container(id, None).await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let docker = docker_client();
        let container = RunnableImage::from(GenericImage::new("hello-world", "latest"))
            .start()
            .await;

        // inspect volume and env
        let container_details = inspect(&docker.bollard, container.id()).await;
        assert_that!(container_details.host_config.unwrap().publish_all_ports)
            .is_equal_to(Some(true));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_only_requested_ports() {
        let docker = docker_client();
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_mapped_port((123, 456))
            .with_mapped_port((555, 888))
            .start()
            .await;

        let container_details = inspect(&docker.bollard, container.id()).await;

        let port_bindings = container_details
            .host_config
            .unwrap()
            .port_bindings
            .unwrap();
        assert_that!(&port_bindings).contains_key(&"456/tcp".into());
        assert_that!(&port_bindings).contains_key(&"888/tcp".into());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_include_network() {
        let docker = docker_client();
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_network("awesome-net-1")
            .start()
            .await;

        let container_details = inspect(&docker.bollard, container.id()).await;
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
        let docker = docker_client();
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_container_name("hello_container")
            .start()
            .await;

        let container_details = inspect(&docker.bollard, container.id()).await;
        assert_that!(container_details.name.unwrap()).ends_with("hello_container");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        let docker = docker_client();
        let hello_world = GenericImage::new("hello-world", "latest");

        {
            let docker = &docker.clone();
            assert!(!network_exists(&docker.bollard, "awesome-net-2").await);

            // creating the first container creates the network
            let _container1 = RunnableImage::from(hello_world.clone())
                .with_network("awesome-net-2")
                .start()
                .await;

            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = RunnableImage::from(hello_world)
                .with_network("awesome-net-2")
                .start()
                .await;

            assert!(network_exists(&docker.bollard, "awesome-net-2").await);
        }

        // client has been dropped, should clean up networks
        assert!(!network_exists(&docker.bollard, "awesome-net-2").await)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_set_shared_memory_size() {
        let docker = docker_client();
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_shm_size(1_000_000)
            .start()
            .await;

        let container_details = inspect(&docker.bollard, container.id()).await;
        let shm_size = container_details.host_config.unwrap().shm_size.unwrap();

        assert_eq!(shm_size, 1_000_000);
    }
}
