use std::collections::HashMap;

use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions},
    models::{HostConfig, PortBinding},
};
use bollard_stubs::models::HostConfigCgroupnsModeEnum;

use crate::{
    core::{
        client::Client,
        mounts::{AccessMode, Mount, MountType},
        network::Network,
        ContainerState,
    },
    CgroupnsMode, ContainerAsync, Image, ImageArgs, RunnableImage,
};

#[async_trait]
/// Helper trait to start containers asynchronously.
///
/// ## Example
///
/// ```rust,no_run
/// use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};
///
/// async fn test_redis() {
///     let container = GenericImage::new("redis", "7.2.4")
///         .with_exposed_port(6379)
///         .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
///         .start()
///         .await;
/// }
/// ```
pub trait AsyncRunner<I: Image> {
    /// Starts the container and returns an instance of `ContainerAsync`.
    async fn start(self) -> ContainerAsync<I>;

    /// Pulls the image from the registry.
    /// Useful if you want to pull the image before starting the container.
    async fn pull_image(self) -> RunnableImage<I>;
}

#[async_trait]
impl<T, I> AsyncRunner<I> for T
where
    T: Into<RunnableImage<I>> + Send,
    I: Image,
{
    async fn start(self) -> ContainerAsync<I> {
        let client = Client::lazy_client().await;
        let runnable_image = self.into();
        let mut create_options: Option<CreateContainerOptions<String>> = None;

        let extra_hosts: Vec<_> = runnable_image
            .hosts()
            .map(|(key, value)| format!("{key}:{value}"))
            .collect();

        let mut config: Config<String> = Config {
            image: Some(runnable_image.descriptor()),
            host_config: Some(HostConfig {
                privileged: Some(runnable_image.privileged()),
                extra_hosts: Some(extra_hosts),
                cgroupns_mode: runnable_image.cgroupns_mode().map(|mode| mode.into()),
                userns_mode: runnable_image.userns_mode(),
                ..Default::default()
            }),
            ..Default::default()
        };

        // shared memory
        if let Some(bytes) = runnable_image.shm_size() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.shm_size = Some(bytes as i64);
                host_config
            });
        }

        // create network and add it to container creation
        let network = if let Some(network) = runnable_image.network() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.network_mode = Some(network.to_string());
                host_config
            });
            Network::new(network, client.clone()).await
        } else {
            None
        };

        // name of the container
        if let Some(name) = runnable_image.container_name() {
            create_options = Some(CreateContainerOptions {
                name: name.to_owned(),
                platform: None,
            })
        }

        // handle environment variables
        let envs: Vec<String> = runnable_image
            .env_vars()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        config.env = Some(envs);

        // mounts and volumes
        let mounts: Vec<_> = runnable_image.mounts().map(Into::into).collect();

        if !mounts.is_empty() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.mounts = Some(mounts);
                host_config
            });
        }

        // entrypoint
        if let Some(entrypoint) = runnable_image.entrypoint() {
            config.entrypoint = Some(vec![entrypoint]);
        }

        let is_container_networked = runnable_image
            .network()
            .as_ref()
            .map(|network| network.starts_with("container:"))
            .unwrap_or(false);

        // expose ports
        if !is_container_networked {
            let mapped_ports = runnable_image
                .ports()
                .as_ref()
                .map(|ports| ports.iter().map(|p| p.internal).collect::<Vec<_>>())
                .unwrap_or_default();

            let ports_to_expose = runnable_image
                .expose_ports()
                .iter()
                .copied()
                .chain(mapped_ports)
                .map(|p| (format!("{p}/tcp"), HashMap::new()))
                .collect();

            // exposed ports of the image + mapped ports
            config.exposed_ports = Some(ports_to_expose);
        }

        // ports
        if runnable_image.ports().is_some() {
            let empty: Vec<_> = Vec::new();
            let bindings = runnable_image
                .ports()
                .as_ref()
                .unwrap_or(&empty)
                .iter()
                .map(|p| {
                    (
                        format!("{}/tcp", p.internal),
                        Some(vec![PortBinding {
                            host_ip: None,
                            host_port: Some(p.local.to_string()),
                        }]),
                    )
                });

            config.host_config = config.host_config.map(|mut host_config| {
                host_config.port_bindings = Some(bindings.collect());
                host_config
            });
        } else if !is_container_networked {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.publish_all_ports = Some(true);
                host_config
            });
        }

        let args = runnable_image
            .args()
            .clone()
            .into_iterator()
            .collect::<Vec<String>>();
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
                    client.pull_image(&runnable_image.descriptor()).await;
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
        if client.config.command() == crate::core::env::Command::Remove {
            crate::watchdog::register(container_id.clone());
        }

        client
            .bollard
            .start_container::<String>(&container_id, None)
            .await
            .unwrap();

        let container =
            ContainerAsync::new(container_id, client.clone(), runnable_image, network).await;

        for cmd in container
            .image()
            .exec_after_start(ContainerState::new(container.ports().await))
        {
            container.exec(cmd).await;
        }

        container
    }

    async fn pull_image(self) -> RunnableImage<I> {
        let runnable_image = self.into();
        let client = Client::lazy_client().await;
        client.pull_image(&runnable_image.descriptor()).await;

        runnable_image
    }
}

impl From<&Mount> for bollard::models::Mount {
    fn from(mount: &Mount) -> Self {
        let mount_type = match mount.mount_type() {
            MountType::Bind => bollard::models::MountTypeEnum::BIND,
            MountType::Volume => bollard::models::MountTypeEnum::VOLUME,
            MountType::Tmpfs => bollard::models::MountTypeEnum::TMPFS,
        };

        let is_read_only = matches!(mount.access_mode(), AccessMode::ReadOnly);

        Self {
            target: mount.target().map(str::to_string),
            source: mount.source().map(str::to_string),
            typ: Some(mount_type),
            read_only: Some(is_read_only),
            ..Default::default()
        }
    }
}

impl From<CgroupnsMode> for HostConfigCgroupnsModeEnum {
    fn from(value: CgroupnsMode) -> Self {
        match value {
            CgroupnsMode::Host => Self::HOST,
            CgroupnsMode::Private => Self::PRIVATE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::WaitFor, images::generic::GenericImage};

    #[tokio::test]
    async fn async_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
        let client = Client::lazy_client().await;
        let container = RunnableImage::from(GenericImage::new("hello-world", "latest"))
            .start()
            .await;

        let container_details = client.inspect(container.id()).await;
        let publish_ports = container_details
            .host_config
            .unwrap()
            .publish_all_ports
            .unwrap();
        assert!(publish_ports, "publish_all_ports must be `true`");
    }

    #[tokio::test]
    async fn async_run_command_should_map_exposed_port() {
        let image = GenericImage::new("simple_web_server", "latest")
            .with_exposed_port(5000)
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));
        let container = image.start().await;
        container.get_host_port_ipv4(5000).await;
    }

    #[tokio::test]
    async fn async_run_command_should_expose_only_requested_ports() {
        let client = Client::lazy_client().await;
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_mapped_port((123, 456))
            .with_mapped_port((555, 888))
            .start()
            .await;

        let container_details = client.inspect(container.id()).await;

        let port_bindings = container_details
            .host_config
            .unwrap()
            .port_bindings
            .unwrap();
        assert!(port_bindings.contains_key("456/tcp"));
        assert!(port_bindings.contains_key("888/tcp"));
    }

    #[tokio::test]
    async fn async_run_command_should_include_network() {
        let client = Client::lazy_client().await;
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_network("awesome-net-1")
            .start()
            .await;

        let container_details = client.inspect(container.id()).await;
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

    #[tokio::test]
    async fn async_run_command_should_include_name() {
        let client = Client::lazy_client().await;
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_container_name("async_hello_container")
            .start()
            .await;

        let container_details = client.inspect(container.id()).await;
        let container_name = container_details.name.unwrap();
        assert!(container_name.ends_with("async_hello_container"));
    }

    #[tokio::test]
    async fn async_should_create_network_if_image_needs_it_and_drop_it_in_the_end() {
        let hello_world = GenericImage::new("hello-world", "latest");

        {
            let client = Client::lazy_client().await;
            assert!(!client.network_exists("awesome-net-2").await);

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

            assert!(client.network_exists("awesome-net-2").await);
        }

        // containers have been dropped, should clean up networks
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let client = Client::lazy_client().await;
        assert!(!client.network_exists("awesome-net-2").await)
    }

    #[tokio::test]
    async fn async_should_rely_on_network_mode_when_no_network_provided_and_settings_bridge_empty()
    {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = RunnableImage::from(web_server.clone()).start().await;

        assert!(!container
            .get_bridge_ip_address()
            .await
            .to_string()
            .is_empty())
    }

    #[tokio::test]
    async fn async_should_rely_on_network_mode_when_network_is_provided_and_settings_bridge_empty()
    {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = RunnableImage::from(web_server.clone())
            .with_network("bridge")
            .start()
            .await;

        assert!(!container
            .get_bridge_ip_address()
            .await
            .to_string()
            .is_empty())
    }

    #[tokio::test]
    #[should_panic]
    async fn async_should_panic_when_non_bridged_network_selected() {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = RunnableImage::from(web_server.clone())
            .with_network("host")
            .start()
            .await;

        container.get_bridge_ip_address().await;
    }

    #[tokio::test]
    async fn async_run_command_should_set_shared_memory_size() {
        let client = Client::lazy_client().await;
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_shm_size(1_000_000)
            .start()
            .await;

        let container_details = client.inspect(container.id()).await;
        let shm_size = container_details.host_config.unwrap().shm_size.unwrap();

        assert_eq!(shm_size, 1_000_000);
    }

    #[tokio::test]
    async fn async_run_command_should_include_privileged() {
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_privileged(true)
            .start()
            .await;

        let client = Client::lazy_client().await;
        let container_details = client.inspect(container.id()).await;

        let privileged = container_details.host_config.unwrap().privileged.unwrap();
        assert!(privileged, "privileged must be `true`");
    }

    #[tokio::test]
    async fn async_run_command_should_have_host_cgroupns_mode() {
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_cgroupns_mode(CgroupnsMode::Host)
            .start()
            .await;

        let client = Client::lazy_client().await;
        let container_details = client.inspect(container.id()).await;

        let cgroupns_mode = container_details
            .host_config
            .unwrap()
            .cgroupns_mode
            .unwrap();

        assert_eq!(
            HostConfigCgroupnsModeEnum::HOST,
            cgroupns_mode,
            "cgroupns mode must be `host`"
        );
    }

    #[tokio::test]
    async fn async_run_command_should_have_private_cgroupns_mode() {
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_cgroupns_mode(CgroupnsMode::Private)
            .start()
            .await;

        let client = Client::lazy_client().await;
        let container_details = client.inspect(container.id()).await;

        let cgroupns_mode = container_details
            .host_config
            .unwrap()
            .cgroupns_mode
            .unwrap();

        assert_eq!(
            HostConfigCgroupnsModeEnum::PRIVATE,
            cgroupns_mode,
            "cgroupns mode must be `private`"
        );
    }

    #[tokio::test]
    async fn async_run_command_should_have_host_userns_mode() {
        let image = GenericImage::new("hello-world", "latest");
        let container = RunnableImage::from(image)
            .with_userns_mode("host")
            .start()
            .await;

        let client = Client::lazy_client().await;
        let container_details = client.inspect(container.id()).await;

        let userns_mode = container_details.host_config.unwrap().userns_mode.unwrap();
        assert_eq!("host", userns_mode, "userns mode must be `host`");
    }
}
