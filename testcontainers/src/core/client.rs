use std::{io, sync::Arc};

use bollard::{
    auth::DockerCredentials,
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions},
    errors::Error as BollardError,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    image::CreateImageOptions,
    network::{CreateNetworkOptions, InspectNetworkOptions},
    Docker,
};
use bollard_stubs::models::{ContainerInspectResponse, ExecInspectResponse, Network};
use futures::{StreamExt, TryStreamExt};
use tokio::sync::OnceCell;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::core::{
    client::exec::ExecResult,
    env,
    env::ConfigurationError,
    logs::{LogSource, LogStreamAsync},
    ports::{PortMappingError, Ports},
};

mod bollard_client;
mod exec;
mod factory;

static IN_A_CONTAINER: OnceCell<bool> = OnceCell::const_new();

// See https://github.com/docker/docker/blob/a9fa38b1edf30b23cae3eade0be48b3d4b1de14b/daemon/initlayer/setup_unix.go#L25
// and Java impl: https://github.com/testcontainers/testcontainers-java/blob/994b385761dde7d832ab7b6c10bc62747fe4b340/core/src/main/java/org/testcontainers/dockerclient/DockerClientConfigUtils.java#L16C5-L17
async fn is_in_container() -> bool {
    *IN_A_CONTAINER
        .get_or_init(|| async { tokio::fs::metadata("/.dockerenv").await.is_ok() })
        .await
}

/// Error type for client operations.
// Mostly wrapper around bollard errors, because they are not very user-friendly.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("failed to initialize a docker client: {0}")]
    Init(BollardError),
    #[error("configuration error: {0}")]
    Configuration(#[from] ConfigurationError),
    #[error("invalid docker host: {0}")]
    InvalidDockerHost(String),
    #[error("failed to pull the image '{descriptor}', error: {err}")]
    PullImage {
        descriptor: String,
        err: BollardError,
    },
    #[error("failed to map ports: {0}")]
    PortMapping(#[from] PortMappingError),

    #[error("failed to create a container: {0}")]
    CreateContainer(BollardError),
    #[error("failed to remove a container: {0}")]
    RemoveContainer(BollardError),
    #[error("failed to start a container: {0}")]
    StartContainer(BollardError),
    #[error("failed to stop a container: {0}")]
    StopContainer(BollardError),
    #[error("failed to inspect a container: {0}")]
    InspectContainer(BollardError),

    #[error("failed to create a network: {0}")]
    CreateNetwork(BollardError),
    #[error("failed to inspect a network: {0}")]
    InspectNetwork(BollardError),
    #[error("failed to list networks: {0}")]
    ListNetworks(BollardError),
    #[error("failed to remove a network: {0}")]
    RemoveNetwork(BollardError),

    #[error("failed to initialize exec command: {0}")]
    InitExec(BollardError),
    #[error("failed to inspect exec command: {0}")]
    InspectExec(BollardError),
}

/// The internal client.
pub(crate) struct Client {
    pub(crate) config: env::Config,
    bollard: Docker,
}

impl Client {
    async fn new() -> Result<Client, ClientError> {
        let config = env::Config::load::<env::Os>().await?;
        let bollard = bollard_client::init(&config).map_err(ClientError::Init)?;

        Ok(Client { config, bollard })
    }

    pub(crate) fn stdout_logs(&self, id: &str, follow: bool) -> LogStreamAsync {
        self.logs(id, LogSource::StdOut, follow)
    }

    pub(crate) fn stderr_logs(&self, id: &str, follow: bool) -> LogStreamAsync {
        self.logs(id, LogSource::StdErr, follow)
    }

    pub(crate) async fn ports(&self, id: &str) -> Result<Ports, ClientError> {
        let ports = self
            .inspect(id)
            .await?
            .network_settings
            .unwrap_or_default()
            .ports
            .map(Ports::try_from)
            .transpose()?
            .unwrap_or_default();

        Ok(ports)
    }

    pub(crate) async fn inspect(&self, id: &str) -> Result<ContainerInspectResponse, ClientError> {
        self.bollard
            .inspect_container(id, None)
            .await
            .map_err(ClientError::InspectContainer)
    }

    pub(crate) async fn rm(&self, id: &str) -> Result<(), ClientError> {
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
            .map_err(ClientError::RemoveContainer)
    }

    pub(crate) async fn stop(&self, id: &str) -> Result<(), ClientError> {
        self.bollard
            .stop_container(id, None)
            .await
            .map_err(ClientError::StopContainer)
    }

    pub(crate) async fn start(&self, id: &str) -> Result<(), ClientError> {
        self.bollard
            .start_container::<String>(id, None)
            .await
            .map_err(ClientError::Init)
    }

    pub(crate) async fn exec(
        &self,
        container_id: &str,
        cmd: Vec<String>,
    ) -> Result<ExecResult, ClientError> {
        let config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .bollard
            .create_exec(container_id, config)
            .await
            .map_err(ClientError::InitExec)?;

        let res = self
            .bollard
            .start_exec(
                &exec.id,
                Some(StartExecOptions {
                    detach: false,
                    tty: false,
                    output_capacity: None,
                }),
            )
            .await
            .map_err(ClientError::InitExec)?;

        match res {
            StartExecResults::Attached { output, .. } => {
                let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel();
                let (stderr_tx, stderr_rx) = tokio::sync::mpsc::unbounded_channel();

                tokio::spawn(async move {
                    macro_rules! handle_error {
                        ($res:expr) => {
                            if let Err(err) = $res {
                                log::debug!(
                                    "Receiver has been dropped, stop producing messages: {}",
                                    err
                                );
                                break;
                            }
                        };
                    }
                    let mut output = output;
                    while let Some(chunk) = output.next().await {
                        match chunk {
                            Ok(LogOutput::StdOut { message }) => {
                                handle_error!(stdout_tx.send(Ok(message)));
                            }
                            Ok(LogOutput::StdErr { message }) => {
                                handle_error!(stderr_tx.send(Ok(message)));
                            }
                            Err(err) => {
                                let err = Arc::new(err);
                                handle_error!(stdout_tx
                                    .send(Err(io::Error::new(io::ErrorKind::Other, err.clone()))));
                                handle_error!(
                                    stderr_tx.send(Err(io::Error::new(io::ErrorKind::Other, err)))
                                );
                            }
                            Ok(_) => {
                                unreachable!("only stdout and stderr are supported")
                            }
                        }
                    }
                });

                let stdout = LogStreamAsync::new(UnboundedReceiverStream::new(stdout_rx).boxed())
                    .enable_cache();
                let stderr = LogStreamAsync::new(UnboundedReceiverStream::new(stderr_rx).boxed())
                    .enable_cache();

                Ok(ExecResult {
                    id: exec.id,
                    stdout,
                    stderr,
                })
            }
            StartExecResults::Detached => unreachable!("detach is false"),
        }
    }

    pub(crate) async fn inspect_exec(
        &self,
        exec_id: &str,
    ) -> Result<ExecInspectResponse, ClientError> {
        self.bollard
            .inspect_exec(exec_id)
            .await
            .map_err(ClientError::InspectExec)
    }

    fn logs(&self, container_id: &str, log_source: LogSource, follow: bool) -> LogStreamAsync {
        let options = LogsOptions {
            follow,
            stdout: log_source.is_stdout(),
            stderr: log_source.is_stderr(),
            tail: "all".to_owned(),
            ..Default::default()
        };

        let stream = self
            .bollard
            .logs(container_id, Some(options))
            .map_ok(|chunk| chunk.into_bytes())
            .map_err(|err| match err {
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 404,
                    message,
                } => io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("Docker container has been dropped: {}", message),
                ),
                bollard::errors::Error::IOError { err } => err,
                err => io::Error::new(io::ErrorKind::Other, err),
            })
            .boxed();
        LogStreamAsync::new(stream)
    }

    /// Creates a network with given name and returns an ID
    pub(crate) async fn create_network(&self, name: &str) -> Result<Option<String>, ClientError> {
        let network = self
            .bollard
            .create_network(CreateNetworkOptions {
                name: name.to_owned(),
                check_duplicate: true,
                ..Default::default()
            })
            .await
            .map_err(ClientError::CreateNetwork)?;

        Ok(network.id)
    }

    /// Inspects a network
    pub(crate) async fn inspect_network(&self, name: &str) -> Result<Network, ClientError> {
        self.bollard
            .inspect_network(name, Some(InspectNetworkOptions::<String>::default()))
            .await
            .map_err(ClientError::InspectNetwork)
    }

    pub(crate) async fn create_container(
        &self,
        options: Option<CreateContainerOptions<String>>,
        config: Config<String>,
    ) -> Result<String, ClientError> {
        self.bollard
            .create_container(options.clone(), config.clone())
            .await
            .map(|res| res.id)
            .map_err(ClientError::CreateContainer)
    }

    pub(crate) async fn start_container(&self, container_id: &str) -> Result<(), ClientError> {
        self.bollard
            .start_container::<String>(container_id, None)
            .await
            .map_err(ClientError::StartContainer)
    }

    pub(crate) async fn pull_image(&self, descriptor: &str) -> Result<(), ClientError> {
        let pull_options = Some(CreateImageOptions {
            from_image: descriptor,
            ..Default::default()
        });
        let credentials = self.credentials_for_image(descriptor).await;
        let mut pulling = self.bollard.create_image(pull_options, None, credentials);
        while let Some(result) = pulling.next().await {
            result.map_err(|err| ClientError::PullImage {
                descriptor: descriptor.to_string(),
                err,
            })?;
        }
        Ok(())
    }

    pub(crate) async fn network_exists(&self, network: &str) -> Result<bool, ClientError> {
        let networks = self
            .bollard
            .list_networks::<String>(None)
            .await
            .map_err(ClientError::ListNetworks)?;

        Ok(networks
            .iter()
            .any(|i| matches!(&i.name, Some(name) if name == network)))
    }

    pub(crate) async fn remove_network(&self, network: &str) -> Result<(), ClientError> {
        self.bollard
            .remove_network(network)
            .await
            .map_err(ClientError::RemoveNetwork)
    }

    pub(crate) async fn docker_hostname(&self) -> Result<url::Host, ClientError> {
        let docker_host = self.config.docker_host();
        match docker_host.scheme() {
            "tcp" | "http" | "https" => {
                docker_host
                    .host()
                    .map(|host| host.to_owned())
                    .ok_or_else(|| {
                        ConfigurationError::InvalidDockerHost(docker_host.to_string()).into()
                    })
            }
            "unix" | "npipe" => {
                if is_in_container().await {
                    let host = self
                        .bollard
                        .inspect_network::<String>("bridge", None)
                        .await
                        .ok()
                        .and_then(|net| net.ipam)
                        .and_then(|ipam| ipam.config)
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|ipam_cfg| ipam_cfg.gateway)
                        .next()
                        .filter(|gateway| !gateway.trim().is_empty())
                        .unwrap_or_else(|| "localhost".to_string());

                    url::Host::parse(&host)
                        .map_err(|_| ConfigurationError::InvalidDockerHost(host).into())
                } else {
                    Ok(url::Host::Domain("localhost".to_string()))
                }
            }
            _ => unreachable!("docker host is already validated in the config"),
        }
    }

    async fn credentials_for_image(&self, descriptor: &str) -> Option<DockerCredentials> {
        let auth_config = self.config.docker_auth_config()?.to_string();
        let (server, _) = descriptor.split_once('/')?;

        // `docker_credential` uses blocking API, thus we spawn blocking task to prevent executor from being blocked
        let cloned_server = server.to_string();
        let credentials = tokio::task::spawn_blocking(move || {
            docker_credential::get_credential_from_reader(auth_config.as_bytes(), &cloned_server)
                .ok()
        })
        .await
        .ok()
        .flatten()?;

        let bollard_credentials = match credentials {
            docker_credential::DockerCredential::IdentityToken(token) => DockerCredentials {
                identitytoken: Some(token),
                serveraddress: Some(server.to_string()),
                ..DockerCredentials::default()
            },
            docker_credential::DockerCredential::UsernamePassword(username, password) => {
                DockerCredentials {
                    username: Some(username),
                    password: Some(password),
                    serveraddress: Some(server.to_string()),
                    ..DockerCredentials::default()
                }
            }
        };

        Some(bollard_credentials)
    }
}
