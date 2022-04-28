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
    pub async fn run<I: Image>(&self, image: impl Into<RunnableImage<I>>) -> ContainerAsync<'_, I> {
        let image = image.into();
        let mut create_options: Option<CreateContainerOptions<String>> = None;
        let mut config: Config<String> = Config {
            image: Some(image.descriptor()),
            host_config: Some(HostConfig::default()),
            ..Default::default()
        };

        // Create network and add it to container creation
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
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        config.env = Some(envs);

        // volumes
        let vols: HashMap<String, HashMap<(), ()>> = image
            .volumes()
            .into_iter()
            .map(|(orig, dest)| (format!("{}:{}", orig, dest), HashMap::new()))
            .collect();
        config.volumes = Some(vols);

        // entrypoint
        if let Some(entrypoint) = image.entrypoint() {
            config.entrypoint = Some(vec![entrypoint]);
        }

        // ports
        if let Some(ports) = image.ports() {
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
        let id = {
            match create_result {
                Ok(container) => container.id,
                Err(bollard::errors::Error::DockerResponseNotFoundError { message: _ }) => {
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
            crate::watchdog::Watchdog::register(container_id.clone());
        }

        self.inner
            .bollard
            .start_container::<String>(&id, None)
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
    use crate::images::{generic::GenericImage, hello_world::HelloWorld};
    use spectral::prelude::*;

    async fn inspect(client: &bollard::Docker, id: &str) -> ContainerInspectResponse {
        client.inspect_container(id, None).await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let docker = Http::new();
        let image = HelloWorld::default();
        let container = docker.run(image).await;

        // inspect volume and env
        let container_details = inspect(&docker.inner.bollard, container.id()).await;
        assert_that!(container_details.host_config.unwrap().publish_all_ports)
            .is_equal_to(Some(true));
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
        assert_that!(&port_bindings).contains_key(&"456/tcp".into());
        assert_that!(&port_bindings).contains_key(&"888/tcp".into());
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
            "Networks is {:?}",
            networks
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_run_command_should_include_name() {
        let docker = Http::new();
        let image = GenericImage::new("hello-world", "latest");
        let image = RunnableImage::from(image).with_container_name("hello_container");
        let container = docker.run(image).await;

        let container_details = inspect(&docker.inner.bollard, container.id()).await;
        assert_that!(container_details.name.unwrap()).ends_with("hello_container");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        let client = bollard::Docker::connect_with_http_defaults().unwrap();

        {
            let docker = Http::new();
            assert!(!network_exists(&client, "awesome-net-2").await);

            // creating the first container creates the network
            let _container1 = docker
                .run(RunnableImage::from(HelloWorld::default()).with_network("awesome-net-2"))
                .await;

            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = docker
                .run(RunnableImage::from(HelloWorld::default()).with_network("awesome-net-2"))
                .await;

            assert!(network_exists(&client, "awesome-net-2").await);
        }

        // client has been dropped, should clean up networks
        assert!(!network_exists(&client, "awesome-net-2").await)
    }
}
