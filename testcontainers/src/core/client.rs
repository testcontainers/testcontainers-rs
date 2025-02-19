use std::{
    collections::HashMap,
    io::{self},
    str::FromStr,
};

use bollard::{
    auth::DockerCredentials,
    container::{
        Config, CreateContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
        RemoveContainerOptions, UploadToContainerOptions,
    },
    errors::Error as BollardError,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    image::CreateImageOptions,
    network::{CreateNetworkOptions, InspectNetworkOptions},
    Docker,
};
use bollard_stubs::models::{ContainerInspectResponse, ExecInspectResponse, Network};
use futures::{StreamExt, TryStreamExt};
use tokio::sync::OnceCell;
use url::Url;

use crate::core::{
    client::exec::ExecResult,
    copy::{CopyToContainer, CopyToContainerError},
    env,
    env::ConfigurationError,
    logs::{
        stream::{LogStream, RawLogStream},
        LogFrame, LogSource, WaitingStreamWrapper,
    },
    ports::{PortMappingError, Ports},
};

mod bollard_client;
mod exec;
mod factory;

pub use factory::docker_client_instance;

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

    #[error("failed to list containers: {0}")]
    ListContainers(BollardError),
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
    #[error("failed to upload data to container: {0}")]
    UploadToContainerError(BollardError),
    #[error("failed to prepare data for copy-to-container: {0}")]
    CopyToContainerError(CopyToContainerError),
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

    pub(crate) fn stdout_logs(&self, id: &str, follow: bool) -> RawLogStream {
        self.logs_stream(id, Some(LogSource::StdOut), follow)
            .into_stdout()
    }

    pub(crate) fn stderr_logs(&self, id: &str, follow: bool) -> RawLogStream {
        self.logs_stream(id, Some(LogSource::StdErr), follow)
            .into_stderr()
    }

    pub(crate) fn logs(&self, id: &str, follow: bool) -> LogStream {
        self.logs_stream(id, None, follow)
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
                let (stdout, stderr) = LogStream::from(output).split().await;
                let stdout = WaitingStreamWrapper::new(stdout).enable_cache();
                let stderr = WaitingStreamWrapper::new(stderr).enable_cache();

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

    fn logs_stream(
        &self,
        container_id: &str,
        source_filter: Option<LogSource>,
        follow: bool,
    ) -> LogStream {
        let options = LogsOptions {
            follow,
            stdout: source_filter.map(LogSource::is_stdout).unwrap_or(true),
            stderr: source_filter.map(LogSource::is_stderr).unwrap_or(true),
            tail: "all".to_owned(),
            ..Default::default()
        };

        self.bollard.logs(container_id, Some(options)).into()
    }

    /// Creates a network with given name and returns an ID
    pub(crate) async fn create_network(&self, name: &str) -> Result<String, ClientError> {
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

    pub(crate) async fn copy_to_container(
        &self,
        container_id: impl Into<String>,
        copy_to_container: &CopyToContainer,
    ) -> Result<(), ClientError> {
        let container_id: String = container_id.into();

        let options = UploadToContainerOptions {
            path: "/".to_string(),
            no_overwrite_dir_non_dir: "false".into(),
        };

        let tar = copy_to_container
            .tar()
            .await
            .map_err(ClientError::CopyToContainerError)?;

        self.bollard
            .upload_to_container::<String>(&container_id, Some(options), tar)
            .await
            .map_err(ClientError::UploadToContainerError)
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
        let docker_host = &self.config.docker_host();
        let docker_host_url = Url::from_str(docker_host)
            .map_err(|e| ConfigurationError::InvalidDockerHost(e.to_string()))?;

        match docker_host_url.scheme() {
            "tcp" | "http" | "https" => docker_host_url
                .host()
                .map(|host| host.to_owned())
                .ok_or_else(|| {
                    ConfigurationError::InvalidDockerHost(docker_host.to_string()).into()
                }),
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

    /// Get the `id` of the first running container whose `name`, `network`,
    /// and `labels` match the supplied values
    #[cfg_attr(not(feature = "reusable-containers"), allow(dead_code))]
    pub(crate) async fn get_running_container_id(
        &self,
        name: Option<&str>,
        network: Option<&str>,
        labels: &HashMap<String, String>,
    ) -> Result<Option<String>, ClientError> {
        let filters = [
            Some(("status".to_string(), vec!["running".to_string()])),
            name.map(|value| ("name".to_string(), vec![value.to_string()])),
            network.map(|value| ("network".to_string(), vec![value.to_string()])),
            Some((
                "label".to_string(),
                labels
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect(),
            )),
        ]
        .into_iter()
        .flatten()
        .collect::<HashMap<_, _>>();

        let options = Some(ListContainersOptions {
            all: false,
            size: false,
            limit: None,
            filters: filters.clone(),
        });

        let containers = self
            .bollard
            .list_containers(options)
            .await
            .map_err(ClientError::ListContainers)?;

        if containers.len() > 1 {
            log::warn!(
                "Found {} containers matching filters: {:?}",
                containers.len(),
                filters
            );
        }

        Ok(containers
            .into_iter()
            // Use `max_by_key()` instead of `next()` to ensure we're
            // returning the id of most recently created container.
            .max_by_key(|container| container.created.unwrap_or(i64::MIN))
            .and_then(|container| container.id))
    }
}

impl<BS> From<BS> for LogStream
where
    BS: futures::Stream<Item = Result<LogOutput, BollardError>> + Send + 'static,
{
    fn from(stream: BS) -> Self {
        let stream = stream
            .try_filter_map(|chunk| async {
                match chunk {
                    LogOutput::StdErr { message } => Ok(Some(LogFrame::StdErr(message))),
                    LogOutput::StdOut { message } => Ok(Some(LogFrame::StdOut(message))),
                    // We only interested in stdout and stderr. Docker may return stdin in some
                    // cases, but we don't need it as we have only one-way communication.
                    LogOutput::StdIn { .. } | LogOutput::Console { .. } => Ok(None),
                }
            })
            .map_err(|err| match err {
                BollardError::DockerResponseServerError {
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
        LogStream::new(stream)
    }
}
