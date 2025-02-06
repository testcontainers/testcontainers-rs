use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions},
    models::{HostConfig, PortBinding},
};
use bollard_stubs::models::{HostConfigCgroupnsModeEnum, ResourcesUlimits};

use crate::{
    core::{
        client::{Client, ClientError},
        copy::CopyToContainer,
        error::{Result, WaitContainerError},
        mounts::{AccessMode, Mount, MountType},
        network::Network,
        CgroupnsMode, ContainerState,
    },
    ContainerAsync, ContainerRequest, Image,
};

const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
#[cfg(feature = "reusable-containers")]
static TESTCONTAINERS_SESSION_ID: std::sync::OnceLock<ulid::Ulid> = std::sync::OnceLock::new();

#[doc(hidden)]
/// A unique identifier for the currently "active" `testcontainers` "session".
///
/// This identifier is used to ensure that the current "session" does not confuse
/// containers it creates with those created by previous runs of a test suite.
///
/// For reference: without a unique-per-session identifier, containers created by
/// previous test sessions that were marked `reuse`, (or where the test suite was
/// run with the `TESTCONTAINERS_COMMAND` environment variable set to `keep`), that
/// *haven't* been manually cleaned up could be incorrectly returned from methods
/// like [`Client::get_running_container_id`](Client::get_running_container_id),
/// as the container name, labels, and network would all still match.
#[cfg(feature = "reusable-containers")]
pub(crate) fn session_id() -> &'static ulid::Ulid {
    TESTCONTAINERS_SESSION_ID.get_or_init(ulid::Ulid::new)
}

#[async_trait]
/// Helper trait to start containers asynchronously.
///
/// ## Example
///
/// ```rust,no_run
/// use testcontainers::{core::{WaitFor, IntoContainerPort}, runners::AsyncRunner, GenericImage};
///
/// async fn test_redis() {
///     let container = GenericImage::new("redis", "7.2.4")
///         .with_exposed_port(6379.tcp())
///         .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
///         .start()
///         .await;
/// }
/// ```
pub trait AsyncRunner<I: Image> {
    /// Starts the container and returns an instance of `ContainerAsync`.
    async fn start(self) -> Result<ContainerAsync<I>>;

    /// Pulls the image from the registry.
    /// Useful if you want to pull the image before starting the container.
    async fn pull_image(self) -> Result<ContainerRequest<I>>;
}

#[async_trait]
impl<T, I> AsyncRunner<I> for T
where
    T: Into<ContainerRequest<I>> + Send,
    I: Image,
{
    async fn start(self) -> Result<ContainerAsync<I>> {
        let container_req = self.into();

        let client = Client::lazy_client().await?;
        let mut create_options: Option<CreateContainerOptions<String>> = None;

        let extra_hosts: Vec<_> = container_req
            .hosts()
            .map(|(key, value)| format!("{key}:{value}"))
            .collect();

        let labels = HashMap::<String, String>::from_iter(
            container_req
                .labels()
                .iter()
                .map(|(key, value)| (key.into(), value.into()))
                .chain([
                    (
                        "org.testcontainers.managed-by".into(),
                        "testcontainers".into(),
                    ),
                    #[cfg(feature = "reusable-containers")]
                    {
                        if container_req.reuse() != crate::ReuseDirective::CurrentSession {
                            Default::default()
                        } else {
                            (
                                "org.testcontainers.session-id".to_string(),
                                session_id().to_string(),
                            )
                        }
                    },
                ])
                .filter(|(_, value): &(_, String)| !value.is_empty()),
        );

        #[cfg(feature = "reusable-containers")]
        {
            use crate::ReuseDirective::{Always, CurrentSession};

            if matches!(container_req.reuse(), Always | CurrentSession) {
                if let Some(container_id) = client
                    .get_running_container_id(
                        container_req.container_name().as_deref(),
                        container_req.network().as_deref(),
                        &labels,
                    )
                    .await?
                {
                    let network = if let Some(network) = container_req.network() {
                        Network::new(network, client.clone()).await?
                    } else {
                        None
                    };

                    return Ok(ContainerAsync::construct(
                        container_id,
                        client,
                        container_req,
                        network,
                    ));
                }
            }
        }

        let mut config: Config<String> = Config {
            image: Some(container_req.descriptor()),
            labels: Some(labels),
            host_config: Some(HostConfig {
                privileged: Some(container_req.privileged()),
                extra_hosts: Some(extra_hosts),
                cgroupns_mode: container_req.cgroupns_mode().map(|mode| mode.into()),
                userns_mode: container_req.userns_mode().map(|v| v.to_string()),
                cap_add: container_req.cap_add().cloned(),
                cap_drop: container_req.cap_drop().cloned(),
                ..Default::default()
            }),
            working_dir: container_req.working_dir().map(|dir| dir.to_string()),
            ..Default::default()
        };

        // shared memory
        if let Some(bytes) = container_req.shm_size() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.shm_size = Some(bytes as i64);
                host_config
            });
        }

        // create network and add it to container creation
        let network = if let Some(network) = container_req.network() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.network_mode = Some(network.to_string());
                host_config
            });
            Network::new(network, client.clone()).await?
        } else {
            None
        };

        // name of the container
        if let Some(name) = container_req.container_name() {
            create_options = Some(CreateContainerOptions {
                name: name.to_owned(),
                platform: None,
            })
        }

        // handle environment variables
        let envs: Vec<String> = container_req
            .env_vars()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        config.env = Some(envs);

        // mounts and volumes
        let mounts: Vec<_> = container_req.mounts().map(Into::into).collect();

        if !mounts.is_empty() {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.mounts = Some(mounts);
                host_config
            });
        }

        // entrypoint
        if let Some(entrypoint) = container_req.entrypoint() {
            config.entrypoint = Some(vec![entrypoint.to_string()]);
        }

        let is_container_networked = container_req
            .network()
            .as_ref()
            .map(|network| network.starts_with("container:"))
            .unwrap_or(false);

        // expose ports
        if !is_container_networked {
            let mapped_ports = container_req
                .ports()
                .map(|ports| ports.iter().map(|p| p.container_port).collect::<Vec<_>>())
                .unwrap_or_default();

            let ports_to_expose = container_req
                .expose_ports()
                .iter()
                .copied()
                .chain(mapped_ports)
                .map(|p| (format!("{p}"), HashMap::new()))
                .collect();

            // exposed ports of the image + mapped ports
            config.exposed_ports = Some(ports_to_expose);
        }

        // ports
        if container_req.ports().is_some() {
            let empty: Vec<_> = Vec::new();
            let bindings = container_req.ports().unwrap_or(&empty).iter().map(|p| {
                (
                    format!("{}", p.container_port),
                    Some(vec![PortBinding {
                        host_ip: None,
                        host_port: Some(p.host_port.to_string()),
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

        // resource ulimits
        if let Some(ulimits) = &container_req.ulimits {
            config.host_config = config.host_config.map(|mut host_config| {
                host_config.ulimits = Some(
                    ulimits
                        .iter()
                        .map(|ulimit| ResourcesUlimits {
                            name: ulimit.name.clone(),
                            soft: ulimit.soft,
                            hard: ulimit.hard,
                        })
                        .collect(),
                );
                host_config
            });
        }

        let cmd: Vec<_> = container_req.cmd().map(|v| v.to_string()).collect();
        if !cmd.is_empty() {
            config.cmd = Some(cmd);
        }

        // create the container with options
        let create_result = client
            .create_container(create_options.clone(), config.clone())
            .await;
        let container_id = match create_result {
            Ok(id) => Ok(id),
            Err(ClientError::CreateContainer(
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                },
            )) => {
                client.pull_image(&container_req.descriptor()).await?;
                client.create_container(create_options, config).await
            }
            res => res,
        }?;

        let copy_to_sources: Vec<&CopyToContainer> =
            container_req.copy_to_sources().map(Into::into).collect();

        for copy_to_source in copy_to_sources {
            client
                .copy_to_container(&container_id, copy_to_source)
                .await?;
        }

        #[cfg(feature = "watchdog")]
        if client.config.command() == crate::core::env::Command::Remove {
            crate::watchdog::register(container_id.clone());
        }

        let startup_timeout = container_req
            .startup_timeout()
            .unwrap_or(DEFAULT_STARTUP_TIMEOUT);

        tokio::time::timeout(startup_timeout, async {
            client.start_container(&container_id).await?;

            let container =
                ContainerAsync::new(container_id, client.clone(), container_req, network).await?;

            let state = ContainerState::from_container(&container).await?;
            for cmd in container.image().exec_after_start(state)? {
                container.exec(cmd).await?;
            }

            Ok(container)
        })
        .await
        .map_err(|_| WaitContainerError::StartupTimeout)?
    }

    async fn pull_image(self) -> Result<ContainerRequest<I>> {
        let container_req = self.into();
        let client = Client::lazy_client().await?;
        client.pull_image(&container_req.descriptor()).await?;

        Ok(container_req)
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
    use crate::{
        core::{IntoContainerPort, WaitFor},
        images::generic::GenericImage,
        ImageExt,
    };

    /// Test that all user-supplied labels are added to containers started by `AsyncRunner::start`
    #[tokio::test]
    async fn async_start_should_apply_expected_labels() -> anyhow::Result<()> {
        let mut labels = HashMap::from([
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string()),
            // include a `managed-by` value to guard against future changes
            // inadvertently allowing users to override keys we rely on to
            // internally ensure sane and correct behavior
            (
                "org.testcontainers.managed-by".to_string(),
                "the-time-wizard".to_string(),
            ),
        ]);

        let image = GenericImage::new("hello-world", "latest").with_labels(&labels);

        let container = {
            #[cfg(not(feature = "reusable-containers"))]
            {
                image
            }
            #[cfg(feature = "reusable-containers")]
            {
                image.with_reuse(crate::ReuseDirective::CurrentSession)
            }
        }
        .start()
        .await?;

        let client = Client::lazy_client().await?;

        let container_labels = client
            .inspect(container.id())
            .await?
            .config
            .unwrap_or_default()
            .labels
            .unwrap_or_default();

        // the created labels and container labels shouldn't actually be identical, as the
        // `org.testcontainers.managed-by: testcontainers` label is always unconditionally
        // applied to all containers by `AsyncRunner::start`, with the value `testcontainers`
        // being applied *last* explicitly so that even user-supplied values of the
        // `org.testcontainers.managed-by` key will be overwritten
        assert_ne!(&labels, &container_labels);

        // If we add the expected `managed-by` value though, they should then match
        labels.insert(
            "org.testcontainers.managed-by".to_string(),
            "testcontainers".to_string(),
        );

        #[cfg(feature = "reusable-containers")]
        labels.extend([(
            "org.testcontainers.session-id".to_string(),
            session_id().to_string(),
        )]);

        assert_eq!(labels, container_labels);

        container.rm().await.map_err(anyhow::Error::from)
    }

    #[tokio::test]
    async fn async_run_command_should_expose_all_ports_if_no_explicit_mapping_requested(
    ) -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let container = GenericImage::new("hello-world", "latest").start().await?;

        let container_details = client.inspect(container.id()).await?;
        let publish_ports = container_details
            .host_config
            .expect("HostConfig")
            .publish_all_ports
            .expect("PublishAllPorts");
        assert!(publish_ports, "publish_all_ports must be `true`");
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_map_exposed_port() -> anyhow::Result<()> {
        let image = GenericImage::new("simple_web_server", "latest")
            .with_exposed_port(5000.tcp())
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));
        let container = image.start().await?;
        container
            .get_host_port_ipv4(5000)
            .await
            .expect("Port should be mapped");
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn async_run_command_should_map_exposed_port_udp_sctp() -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let _ = pretty_env_logger::try_init();

        let udp_port = 1000;
        let sctp_port = 2000;

        let generic_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            // Explicitly expose the port, which otherwise would not be available.
            .with_exposed_port(udp_port.udp())
            .with_exposed_port(sctp_port.sctp());

        let container = generic_server.start().await?;
        container.get_host_port_ipv4(udp_port.udp()).await?;
        container.get_host_port_ipv4(sctp_port.sctp()).await?;

        let container_details = client.inspect(container.id()).await?;

        let current_ports_map = container_details
            .network_settings
            .expect("network_settings")
            .ports
            .expect("ports");

        let mut current_ports = current_ports_map.keys().collect::<Vec<&String>>();

        current_ports.sort();

        let mut expected_ports: Vec<&String> = Vec::new();

        let tcp_expected_port = &String::from("80/tcp");
        let sctp_expected_port = &String::from("2000/sctp");
        let udp_expected_port = &String::from("1000/udp");

        expected_ports.push(udp_expected_port);
        expected_ports.push(sctp_expected_port);
        expected_ports.push(tcp_expected_port);

        assert_eq!(current_ports, expected_ports);

        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_expose_only_requested_ports() -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let image = GenericImage::new("hello-world", "latest");
        let container = image
            .with_mapped_port(123, 456.tcp())
            .with_mapped_port(555, 888.tcp())
            .start()
            .await?;

        let container_details = client.inspect(container.id()).await?;

        let port_bindings = container_details
            .host_config
            .expect("HostConfig")
            .port_bindings
            .expect("PortBindings");
        assert!(
            port_bindings.contains_key("456/tcp"),
            "port 456/tcp must be mapped"
        );
        assert!(
            port_bindings.contains_key("888/tcp"),
            "port 888/tcp must be mapped"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn async_run_command_should_map_ports_udp_sctp() -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let _ = pretty_env_logger::try_init();

        let udp_port = 1000;
        let sctp_port = 2000;

        let image = GenericImage::new("hello-world", "latest");
        let container = image
            .with_mapped_port(123, udp_port.udp())
            .with_mapped_port(555, sctp_port.sctp())
            .start()
            .await?;

        let container_details = client.inspect(container.id()).await?;

        let current_ports_map = container_details
            .host_config
            .expect("HostConfig")
            .port_bindings
            .expect("ports");

        let mut current_ports = current_ports_map.keys().collect::<Vec<&String>>();

        current_ports.sort();

        let mut expected_ports: Vec<&String> = Vec::new();

        let sctp_expected_port = &String::from("2000/sctp");
        let udp_expected_port = &String::from("1000/udp");

        expected_ports.push(udp_expected_port);
        expected_ports.push(sctp_expected_port);

        assert_eq!(current_ports, expected_ports);

        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_include_network() -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_network("awesome-net-1").start().await?;

        let container_details = client.inspect(container.id()).await?;
        let networks = container_details
            .network_settings
            .expect("NetworkSettings")
            .networks
            .expect("Networks");

        assert!(
            networks.contains_key("awesome-net-1"),
            "Networks is {networks:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_include_name() -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let image = GenericImage::new("hello-world", "latest");
        let container = image
            .with_container_name("async_hello_container")
            .start()
            .await?;

        let container_details = client.inspect(container.id()).await?;
        let container_name = container_details.name.expect("Name");
        assert!(container_name.ends_with("async_hello_container"));
        Ok(())
    }

    #[tokio::test]
    async fn async_should_create_network_if_image_needs_it_and_drop_it_in_the_end(
    ) -> anyhow::Result<()> {
        let hello_world = GenericImage::new("hello-world", "latest");

        {
            let client = Client::lazy_client().await?;
            assert!(!client.network_exists("awesome-net-2").await?);

            // creating the first container creates the network
            let _container1 = hello_world
                .clone()
                .with_network("awesome-net-2")
                .start()
                .await;

            // creating a 2nd container doesn't fail because check if the network exists already
            let _container2 = hello_world.with_network("awesome-net-2").start().await;

            assert!(client.network_exists("awesome-net-2").await?);
        }

        // containers have been dropped, should clean up networks
        tokio::time::sleep(Duration::from_secs(1)).await;
        let client = Client::lazy_client().await?;
        assert!(!client.network_exists("awesome-net-2").await?);
        Ok(())
    }

    #[tokio::test]
    async fn async_should_rely_on_network_mode_when_network_is_provided_and_settings_bridge_empty(
    ) -> anyhow::Result<()> {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = web_server.clone().with_network("bridge").start().await?;

        assert!(
            !container
                .get_bridge_ip_address()
                .await?
                .to_string()
                .is_empty(),
            "Bridge IP address must not be empty"
        );
        Ok(())
    }

    #[tokio::test]
    async fn async_should_return_error_when_non_bridged_network_selected() -> anyhow::Result<()> {
        let web_server = GenericImage::new("simple_web_server", "latest")
            .with_wait_for(WaitFor::message_on_stdout("server is ready"))
            .with_wait_for(WaitFor::seconds(1));

        let container = web_server.clone().with_network("host").start().await?;

        let res = container.get_bridge_ip_address().await;
        assert!(
            res.is_err(),
            "Getting bridge IP address should fail due to network mode"
        );
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_set_shared_memory_size() -> anyhow::Result<()> {
        let client = Client::lazy_client().await?;
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_shm_size(1_000_000).start().await?;

        let container_details = client.inspect(container.id()).await?;
        let shm_size = container_details
            .host_config
            .expect("HostConfig")
            .shm_size
            .expect("ShmSize");

        assert_eq!(shm_size, 1_000_000);
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_include_privileged() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_privileged(true).start().await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let privileged = container_details
            .host_config
            .expect("HostConfig")
            .privileged
            .expect("Privileged");
        assert!(privileged, "privileged must be `true`");
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_have_cap_add() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let expected_capability = "NET_ADMIN";
        let container = image
            .with_cap_add(expected_capability.to_string())
            .start()
            .await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let capabilities = container_details
            .host_config
            .expect("HostConfig")
            .cap_add
            .expect("CapAdd");

        assert_eq!(
            expected_capability,
            capabilities.first().expect("No capabilities added"),
            "cap_add must contain {expected_capability}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_have_cap_drop() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let expected_capability = "AUDIT_WRITE";
        let container = image
            .with_cap_drop(expected_capability.to_string())
            .start()
            .await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let capabilities = container_details
            .host_config
            .expect("HostConfig")
            .cap_drop
            .expect("CapAdd");

        assert_eq!(
            expected_capability,
            capabilities.first().expect("No capabilities dropped"),
            "cap_drop must contain {expected_capability}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_include_ulimits() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_ulimit("nofile", 123, Some(456)).start().await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let ulimits = container_details
            .host_config
            .expect("HostConfig")
            .ulimits
            .expect("Privileged");

        assert_eq!(ulimits.len(), 1);
        assert_eq!(ulimits[0].name, Some("nofile".into()));
        assert_eq!(ulimits[0].soft, Some(123));
        assert_eq!(ulimits[0].hard, Some(456));
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_have_host_cgroupns_mode() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_cgroupns_mode(CgroupnsMode::Host).start().await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let cgroupns_mode = container_details
            .host_config
            .expect("HostConfig")
            .cgroupns_mode
            .expect("CgroupnsMode");

        assert_eq!(
            HostConfigCgroupnsModeEnum::HOST,
            cgroupns_mode,
            "cgroupns mode must be `host`"
        );
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_have_private_cgroupns_mode() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image
            .with_cgroupns_mode(CgroupnsMode::Private)
            .start()
            .await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let cgroupns_mode = container_details
            .host_config
            .expect("HostConfig")
            .cgroupns_mode
            .expect("CgroupnsMode");

        assert_eq!(
            HostConfigCgroupnsModeEnum::PRIVATE,
            cgroupns_mode,
            "cgroupns mode must be `private`"
        );
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_have_host_userns_mode() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let container = image.with_userns_mode("host").start().await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let userns_mode = container_details
            .host_config
            .expect("HostConfig")
            .userns_mode
            .expect("UsernsMode");
        assert_eq!("host", userns_mode, "userns mode must be `host`");
        Ok(())
    }

    #[tokio::test]
    async fn async_run_command_should_have_working_dir() -> anyhow::Result<()> {
        let image = GenericImage::new("hello-world", "latest");
        let expected_working_dir = "/foo";
        let container = image.with_working_dir(expected_working_dir).start().await?;

        let client = Client::lazy_client().await?;
        let container_details = client.inspect(container.id()).await?;

        let working_dir = container_details
            .config
            .expect("ContainerConfig")
            .working_dir
            .expect("WorkingDir");
        assert_eq!(
            expected_working_dir, &working_dir,
            "working dir must be `foo`"
        );
        Ok(())
    }
}
