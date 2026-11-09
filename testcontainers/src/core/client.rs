use std::{
    collections::HashMap,
    io::{self, Read},
    pin::Pin,
    str::FromStr,
    sync::Arc,
    task::{ready, Context, Poll},
};

use bollard::{
    auth::DockerCredentials,
    body_full,
    container::LogOutput,
    errors::Error as BollardError,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    models::{
        ContainerCreateBody, ContainerInspectResponse, ExecInspectResponse, NetworkCreateRequest,
        NetworkInspect,
    },
    query_parameters::{
        BuildImageOptionsBuilder, BuilderVersion, CreateContainerOptions,
        CreateImageOptionsBuilder, DownloadFromContainerOptionsBuilder, InspectContainerOptions,
        InspectContainerOptionsBuilder, InspectNetworkOptions, InspectNetworkOptionsBuilder,
        ListContainersOptionsBuilder, ListNetworksOptions, LogsOptionsBuilder,
        RemoveContainerOptionsBuilder, StartContainerOptions, StopContainerOptionsBuilder,
        UploadToContainerOptionsBuilder,
    },
    Docker,
};
use ferroid::{base32::Base32UlidExt, id::ULID};
use futures::{StreamExt, TryStreamExt};
use tar::{Archive as TarArchive, EntryType};
use tokio::{
    io::{AsyncRead, AsyncReadExt, ReadBuf, Take},
    sync::{Mutex, OnceCell},
};
use tokio_util::io::StreamReader;
use url::Url;

use crate::core::{
    client::exec::ExecResult,
    copy::{
        CopyFileFromContainer, CopyFromContainerError, CopyToContainer, CopyToContainerCollection,
        CopyToContainerError,
    },
    env::{self, ConfigurationError},
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

type BuildLockMap = Mutex<HashMap<String, Arc<Mutex<()>>>>;
static BUILD_LOCKS: OnceCell<BuildLockMap> = OnceCell::const_new();

async fn get_build_lock(descriptor: &str) -> Arc<Mutex<()>> {
    let locks = BUILD_LOCKS
        .get_or_init(|| async { Mutex::new(HashMap::new()) })
        .await;

    let mut map = locks.lock().await;
    map.entry(descriptor.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

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
    #[error("failed to build the image '{descriptor}', error: {err}")]
    BuildImage {
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
    #[error("failed to pause a container: {0}")]
    PauseContainer(BollardError),
    #[error("failed to unpause/resume a container: {0}")]
    UnpauseContainer(BollardError),
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
    #[error("failed to handle data copied from container: {0}")]
    CopyFromContainerError(CopyFromContainerError),
}

/// Information about a container returned from lookup operations.
#[cfg(feature = "reusable-containers")]
#[derive(Debug, Clone)]
pub(crate) struct ContainerInfo {
    pub id: String,
    pub is_running: bool,
}

/// The internal client.
pub(crate) struct Client {
    pub(crate) config: env::Config,
    bollard: Docker,
}

impl Client {
    async fn new() -> Result<Client, ClientError> {
        let config = env::Config::load::<env::Os>().await?;
        Self::new_with_config(config)
    }

    pub(crate) fn new_with_config(config: env::Config) -> Result<Client, ClientError> {
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

    pub(crate) fn both_std_logs(&self, id: &str, follow: bool) -> RawLogStream {
        self.logs_stream(id, Some(LogSource::BothStd), follow)
            .into_both_std()
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
            .inspect_container(id, None::<InspectContainerOptions>)
            .await
            .map_err(ClientError::InspectContainer)
    }

    // It's used under a feature, but feature gate doesn't make a lot of sense here.
    #[allow(dead_code)]
    pub(crate) async fn list_containers_by_label(
        &self,
        label_key: &str,
        label_value: &str,
    ) -> Result<Vec<bollard::models::ContainerSummary>, ClientError> {
        let filters = HashMap::from([(
            "label".to_string(),
            vec![format!("{}={}", label_key, label_value)],
        )]);

        let options = ListContainersOptionsBuilder::new()
            .all(true)
            .filters(&filters)
            .build();

        self.bollard
            .list_containers(Some(options))
            .await
            .map_err(ClientError::ListContainers)
    }

    pub(crate) async fn rm(&self, id: &str) -> Result<(), ClientError> {
        self.bollard
            .remove_container(
                id,
                Some(
                    RemoveContainerOptionsBuilder::new()
                        .force(true)
                        .v(true)
                        .build(),
                ),
            )
            .await
            .map_err(ClientError::RemoveContainer)
    }

    pub(crate) async fn stop(
        &self,
        id: &str,
        timeout_seconds: Option<i32>,
    ) -> Result<(), ClientError> {
        self.bollard
            .stop_container(
                id,
                timeout_seconds.map(|t| StopContainerOptionsBuilder::new().t(t).build()),
            )
            .await
            .map_err(ClientError::StopContainer)
    }

    pub(crate) async fn start(&self, id: &str) -> Result<(), ClientError> {
        self.bollard
            .start_container(id, None::<StartContainerOptions>)
            .await
            .map_err(ClientError::Init)
    }

    pub(crate) async fn pause(&self, id: &str) -> Result<(), ClientError> {
        self.bollard
            .pause_container(id)
            .await
            .map_err(ClientError::PauseContainer)
    }

    pub(crate) async fn unpause(&self, id: &str) -> Result<(), ClientError> {
        self.bollard
            .unpause_container(id)
            .await
            .map_err(ClientError::UnpauseContainer)
    }

    pub(crate) async fn exec(
        &self,
        container_id: &str,
        cmd: Vec<String>,
        env_vars: std::collections::HashMap<String, String>,
    ) -> Result<ExecResult, ClientError> {
        let env = if env_vars.is_empty() {
            None
        } else {
            Some(
                env_vars
                    .into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect(),
            )
        };

        let config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            env,
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
        let options = LogsOptionsBuilder::new()
            .follow(follow)
            .stdout(
                source_filter
                    .map(LogSource::includes_stdout)
                    .unwrap_or(true),
            )
            .stderr(
                source_filter
                    .map(LogSource::includes_stderr)
                    .unwrap_or(true),
            )
            .tail("all")
            .build();

        self.bollard.logs(container_id, Some(options)).into()
    }

    /// Creates a network with given name and returns an ID
    pub(crate) async fn create_network(&self, name: &str) -> Result<String, ClientError> {
        let network = self
            .bollard
            .create_network(NetworkCreateRequest {
                name: name.to_owned(),
                ..Default::default()
            })
            .await
            .map_err(ClientError::CreateNetwork)?;

        Ok(network.id)
    }

    /// Inspects a network
    pub(crate) async fn inspect_network(&self, name: &str) -> Result<NetworkInspect, ClientError> {
        self.bollard
            .inspect_network(name, Some(InspectNetworkOptionsBuilder::new().build()))
            .await
            .map_err(ClientError::InspectNetwork)
    }

    pub(crate) async fn create_container(
        &self,
        options: Option<CreateContainerOptions>,
        config: ContainerCreateBody,
    ) -> Result<String, ClientError> {
        self.bollard
            .create_container(options, config)
            .await
            .map(|res| res.id)
            .map_err(ClientError::CreateContainer)
    }

    pub(crate) async fn start_container(&self, container_id: &str) -> Result<(), ClientError> {
        self.bollard
            .start_container(container_id, None::<StartContainerOptions>)
            .await
            .map_err(ClientError::StartContainer)
    }

    pub(crate) async fn copy_to_container(
        &self,
        container_id: impl Into<String>,
        copy_to_container: &CopyToContainer,
    ) -> Result<(), ClientError> {
        let container_id: String = container_id.into();

        let options = UploadToContainerOptionsBuilder::new()
            .path("/")
            .no_overwrite_dir_non_dir("false")
            .build();

        let tar = copy_to_container
            .tar()
            .await
            .map_err(ClientError::CopyToContainerError)?;

        self.bollard
            .upload_to_container(&container_id, Some(options), body_full(tar))
            .await
            .map_err(ClientError::UploadToContainerError)
    }

    pub(crate) async fn copy_file_from_container<T>(
        &self,
        container_id: impl AsRef<str>,
        container_path: impl AsRef<str>,
        target: T,
    ) -> Result<T::Output, ClientError>
    where
        T: CopyFileFromContainer,
    {
        let container_id = container_id.as_ref();
        let options = DownloadFromContainerOptionsBuilder::new()
            .path(container_path.as_ref())
            .build();

        let stream = self
            .bollard
            .download_from_container(container_id, Some(options))
            .map_err(io::Error::other);
        let reader = StreamReader::new(stream);
        Self::extract_file_entry(reader, target)
            .await
            .map_err(ClientError::CopyFromContainerError)
    }

    /// Streams the first regular file of the tar `reader` into `target`.
    ///
    /// The synchronous `tar` parser runs over the bytes buffered so far, and more are read from
    /// `reader` until the file's header is complete, parsing again only once the buffer has
    /// doubled so a large header costs linear work. The payload then streams from the buffered
    /// remainder followed by `reader`, and must yield exactly the size its header declares.
    async fn extract_file_entry<R, T>(
        mut reader: R,
        target: T,
    ) -> Result<T::Output, CopyFromContainerError>
    where
        R: AsyncRead + Unpin,
        T: CopyFileFromContainer,
    {
        const READ_CHUNK: usize = 8 * 1024;

        let mut buffered = Vec::new();
        let mut complete = false;
        let mut parse_at = 0;
        let (payload_offset, size) = loop {
            if complete || buffered.len() >= parse_at {
                if let Some(located) = Self::locate_file_entry(&buffered, complete)? {
                    break located;
                }
                parse_at = buffered.len().saturating_mul(2);
            }
            buffered.reserve(READ_CHUNK);
            complete = reader
                .read_buf(&mut buffered)
                .await
                .map_err(CopyFromContainerError::Io)?
                == 0;
        };

        let payload_start = usize::try_from(payload_offset)
            .ok()
            .filter(|&start| start <= buffered.len())
            .ok_or_else(|| {
                CopyFromContainerError::Io(io::Error::other(
                    "tar entry payload starts past the parsed header",
                ))
            })?;
        if let Some(payload_end) = usize::try_from(size)
            .ok()
            .and_then(|size| payload_start.checked_add(size))
        {
            buffered.truncate(payload_end);
        }
        let mut buffered = io::Cursor::new(buffered);
        buffered.set_position(payload_offset);

        let payload = ExactSizeReader::new(AsyncReadExt::chain(buffered, reader), size);
        target.copy_from_reader(payload).await
    }

    /// Finds the first regular file in the tar bytes `data` and returns its payload offset and
    /// size, or `None` when `data` ends before that is known and the stream is not `complete`.
    fn locate_file_entry(
        data: &[u8],
        complete: bool,
    ) -> Result<Option<(u64, u64)>, CopyFromContainerError> {
        let mut source = BufferedArchive {
            remaining: data,
            exhausted: false,
        };
        match Self::scan_entries(&mut source) {
            Ok(located) => Ok(Some(located)),
            Err(_) if source.exhausted && !complete => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Reads tar headers from `source` up to the first regular file and returns its payload
    /// offset and size.
    fn scan_entries(source: impl Read) -> Result<(u64, u64), CopyFromContainerError> {
        let mut archive = TarArchive::new(source);
        for entry in archive.entries().map_err(CopyFromContainerError::Io)? {
            let entry = entry.map_err(CopyFromContainerError::Io)?;
            match entry.header().entry_type() {
                EntryType::GNULongName
                | EntryType::GNULongLink
                | EntryType::XGlobalHeader
                | EntryType::XHeader
                | EntryType::GNUSparse => continue, // skip metadata entries
                EntryType::Directory => return Err(CopyFromContainerError::IsDirectory),
                EntryType::Regular | EntryType::Continuous => {
                    return Ok((entry.raw_file_position(), entry.size()))
                }
                et => return Err(CopyFromContainerError::UnsupportedEntry(et)),
            }
        }

        Err(CopyFromContainerError::EmptyArchive)
    }

    pub(crate) async fn container_is_running(
        &self,
        container_id: &str,
    ) -> Result<bool, ClientError> {
        let options = InspectContainerOptionsBuilder::new().size(false).build();
        let container_info = self
            .bollard
            .inspect_container(container_id, Some(options))
            .await
            .map_err(ClientError::InspectContainer)?;

        if let Some(state) = container_info.state {
            Ok(state.running.unwrap_or_default())
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn container_exit_code(
        &self,
        container_id: &str,
    ) -> Result<Option<i64>, ClientError> {
        let options = InspectContainerOptionsBuilder::new().size(false).build();
        let container_info = self
            .bollard
            .inspect_container(container_id, Some(options))
            .await
            .map_err(ClientError::InspectContainer)?;

        let Some(state) = container_info.state else {
            return Ok(None);
        };
        if state.running == Some(true) {
            return Ok(None);
        }
        Ok(state.exit_code)
    }

    pub(crate) async fn build_image(
        &self,
        descriptor: &str,
        build_context: &CopyToContainerCollection,
        options: crate::core::build::build_options::BuildImageOptions,
    ) -> Result<(), ClientError> {
        if options.skip_if_exists {
            let lock = get_build_lock(descriptor).await;
            let _guard = lock.lock().await;

            match self.bollard.inspect_image(descriptor).await {
                Ok(_) => {
                    log::info!("Image '{}' already exists, skipping build", descriptor);
                    return Ok(());
                }
                Err(BollardError::DockerResponseServerError {
                    status_code: 404, ..
                }) => {
                    log::info!("Image '{}' not found, proceeding with build", descriptor);
                }
                Err(err) => {
                    log::warn!(
                        "Failed to inspect image '{}': {:?}, proceeding with build",
                        descriptor,
                        err
                    );
                }
            }

            self.build_image_impl(descriptor, build_context, options)
                .await
        } else {
            self.build_image_impl(descriptor, build_context, options)
                .await
        }
    }

    async fn build_image_impl(
        &self,
        descriptor: &str,
        build_context: &CopyToContainerCollection,
        options: crate::core::build::build_options::BuildImageOptions,
    ) -> Result<(), ClientError> {
        let tar = build_context
            .tar()
            .await
            .map_err(ClientError::CopyToContainerError)?;

        let session = ULID::now().encode();

        let mut builder = BuildImageOptionsBuilder::new()
            .dockerfile("Dockerfile")
            .t(descriptor)
            .rm(true)
            .nocache(options.no_cache)
            .version(BuilderVersion::BuilderBuildKit)
            .session(session.as_str());

        if !options.build_args.is_empty() {
            builder = builder.buildargs(&options.build_args);
        }

        let build_options = builder.build();

        let credentials = None;

        let mut building =
            self.bollard
                .build_image(build_options, credentials, Some(body_full(tar)));

        while let Some(result) = building.next().await {
            match result {
                Ok(r) => {
                    if let Some(s) = r.stream {
                        log::info!("{}", s);
                    }
                }
                Err(err) => {
                    log::error!("{:?}", err);
                    return Err(ClientError::BuildImage {
                        descriptor: descriptor.into(),
                        err,
                    });
                }
            };
        }

        Ok(())
    }

    pub(crate) async fn pull_image(
        &self,
        descriptor: &str,
        platform: Option<String>,
    ) -> Result<(), ClientError> {
        let pull_options = CreateImageOptionsBuilder::new()
            .from_image(descriptor)
            .platform(
                platform
                    .as_deref()
                    .unwrap_or_else(|| self.config.platform().unwrap_or_default()),
            )
            .build();

        let credentials = self.credentials_for_image(descriptor).await;
        let mut pulling = self
            .bollard
            .create_image(Some(pull_options), None, credentials);
        while let Some(result) = pulling.next().await {
            // if the image pull fails, try to pull the image for linux/amd64 platform instead
            match result {
                Ok(_) => {}
                Err(BollardError::DockerResponseServerError {
                    status_code: _,
                    message: _,
                }) if !matches!(platform.as_deref(), Some("linux/amd64")) => {
                    self.pull_image_linux_amd64(descriptor).await?;
                }
                _ => {
                    // if the linux/amd64 image pull also fails, return the initial error
                    result.map_err(|err| ClientError::PullImage {
                        descriptor: descriptor.to_string(),
                        err,
                    })?;
                }
            };
        }
        Ok(())
    }

    async fn pull_image_linux_amd64(&self, descriptor: &str) -> Result<(), ClientError> {
        let pull_options = CreateImageOptionsBuilder::new()
            .from_image(descriptor)
            .platform("linux/amd64")
            .build();
        let credentials = self.credentials_for_image(descriptor).await;
        let mut pulling = self
            .bollard
            .create_image(Some(pull_options), None, credentials);
        while let Some(result) = pulling.next().await {
            match result {
                Ok(_) => {}
                Err(err) => {
                    return Err(ClientError::PullImage {
                        descriptor: descriptor.to_string(),
                        err,
                    });
                }
            };
        }
        Ok(())
    }

    pub(crate) async fn network_exists(&self, network: &str) -> Result<bool, ClientError> {
        let networks = self
            .bollard
            .list_networks(None::<ListNetworksOptions>)
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
                        .inspect_network("bridge", None::<InspectNetworkOptions>)
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
        let server = resolve_registry(descriptor);

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

    /// Get information about the first container whose `name`, `network`,
    /// and `labels` match the supplied values, regardless of status.
    #[cfg(feature = "reusable-containers")]
    pub(crate) async fn get_container(
        &self,
        name: Option<&str>,
        network: Option<&str>,
        labels: &HashMap<String, String>,
    ) -> Result<Option<ContainerInfo>, ClientError> {
        use bollard::models::ContainerSummaryStateEnum;
        let filters = [
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

        let options = ListContainersOptionsBuilder::new()
            .all(true)
            .size(false)
            .filters(&filters)
            .build();

        let containers = self
            .bollard
            .list_containers(Some(options))
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
            .and_then(|container| {
                container.id.map(|id| {
                    let is_running =
                        matches!(container.state, Some(ContainerSummaryStateEnum::RUNNING));
                    ContainerInfo { id, is_running }
                })
            }))
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
                    format!("Docker container has been dropped: {message}"),
                ),
                bollard::errors::Error::IOError { err } => err,
                err => io::Error::other(err),
            })
            .boxed();
        LogStream::new(stream)
    }
}

/// Docker Hub's canonical registry hostname, used for credential lookups when
/// an image descriptor has no explicit registry prefix (e.g. `postgres:16`).
const DOCKER_HUB_REGISTRY: &str = "index.docker.io";

/// Tar bytes buffered so far, recording whether the parser asked for more than they hold.
struct BufferedArchive<'a> {
    remaining: &'a [u8],
    exhausted: bool,
}

impl Read for BufferedArchive<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining.is_empty() && !buf.is_empty() {
            self.exhausted = true;
            return Ok(0);
        }
        Read::read(&mut self.remaining, buf)
    }
}

/// Yields exactly `size` bytes of `inner`, failing with `UnexpectedEof` if it ends sooner.
struct ExactSizeReader<R> {
    inner: Take<R>,
}

impl<R: AsyncRead + Unpin> ExactSizeReader<R> {
    fn new(inner: R, size: u64) -> Self {
        Self {
            inner: inner.take(size),
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ExactSizeReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let filled = buf.filled().len();
        ready!(Pin::new(&mut this.inner).poll_read(cx, buf))?;
        if buf.filled().len() == filled && buf.remaining() > 0 && this.inner.limit() > 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "tar entry payload ended before its declared size",
            )));
        }
        Poll::Ready(Ok(()))
    }
}

/// Extracts the registry server from a Docker image descriptor.
///
/// Docker image descriptors follow the pattern `[registry/][namespace/]name[:tag]`.
/// When no explicit registry is present, images are pulled from Docker Hub.
///
/// The first path component is treated as a registry hostname only if it contains
/// a `.` or a `:` (port), matching the Docker CLI's resolution logic. Otherwise
/// it is a Docker Hub namespace (e.g. `library`, `confluentinc`).
fn resolve_registry(descriptor: &str) -> &str {
    match descriptor.split_once('/') {
        Some((first, _)) if first.contains('.') || first.contains(':') => first,
        _ => DOCKER_HUB_REGISTRY,
    }
}

#[cfg(test)]
mod tests {
    use bollard::query_parameters::RemoveImageOptions;
    use tokio::io::AsyncWriteExt;

    use super::*;

    #[derive(Debug)]
    struct OsEnvWithPlatformLinuxAmd64;

    impl env::GetEnvValue for OsEnvWithPlatformLinuxAmd64 {
        fn get_env_value(key: &str) -> Option<String> {
            match key {
                "DOCKER_DEFAULT_PLATFORM" => Some("linux/amd64".to_string()),
                _ => env::Os::get_env_value(key),
            }
        }
    }

    #[derive(Debug)]
    struct OsEnvWithPlatformLinux386;

    impl env::GetEnvValue for OsEnvWithPlatformLinux386 {
        fn get_env_value(key: &str) -> Option<String> {
            match key {
                "DOCKER_DEFAULT_PLATFORM" => Some("linux/386".to_string()),
                _ => env::Os::get_env_value(key),
            }
        }
    }

    #[tokio::test]
    // Tehcnically the test is racy if we would want to use image in other tests, but we don't use
    // it usually, so no serial-tests or anything like that is used
    async fn test_client_pull_image_with_platform() -> anyhow::Result<()> {
        const IMAGE: &str = "hello-world:linux";

        let config = env::Config::load::<OsEnvWithPlatformLinuxAmd64>().await?;
        println!("Config platform: {:?}", config.platform());
        let client = Client::new_with_config(config)?;

        // remove image if exists (it may already have another platform variant)
        let credentials = client.credentials_for_image(IMAGE).await;
        let _ = client
            .bollard
            .remove_image(
                IMAGE,
                Option::<RemoveImageOptions>::None,
                credentials.clone(),
            )
            .await;

        client.pull_image(IMAGE, None).await?;

        let image = client.bollard.inspect_image(IMAGE).await?;

        assert_eq!(Some("linux".to_string()), image.os);
        assert_eq!(Some("amd64".to_string()), image.architecture);

        let config = env::Config::load::<OsEnvWithPlatformLinux386>().await?;
        let client = Client::new_with_config(config)?;

        client
            .bollard
            .remove_image(IMAGE, Option::<RemoveImageOptions>::None, credentials)
            .await?;

        client.pull_image(IMAGE, None).await?;

        let image = client.bollard.inspect_image(IMAGE).await?;

        assert_eq!(Some("linux".to_string()), image.os);
        assert_eq!(Some("386".to_string()), image.architecture);

        Ok(())
    }

    #[test]
    fn test_resolve_registry_docker_hub_library_image() {
        assert_eq!(resolve_registry("postgres:16"), DOCKER_HUB_REGISTRY);
        assert_eq!(resolve_registry("redis:7-alpine"), DOCKER_HUB_REGISTRY);
        assert_eq!(resolve_registry("hello-world:linux"), DOCKER_HUB_REGISTRY);
        assert_eq!(resolve_registry("ubuntu"), DOCKER_HUB_REGISTRY);
    }

    #[test]
    fn test_resolve_registry_docker_hub_namespaced_image() {
        assert_eq!(
            resolve_registry("confluentinc/cp-kafka:6.1.1"),
            DOCKER_HUB_REGISTRY
        );
        assert_eq!(resolve_registry("library/postgres:16"), DOCKER_HUB_REGISTRY);
        assert_eq!(resolve_registry("minio/minio:latest"), DOCKER_HUB_REGISTRY);
    }

    #[test]
    fn test_resolve_registry_private_registry() {
        assert_eq!(resolve_registry("ghcr.io/myorg/myimage:latest"), "ghcr.io");
        assert_eq!(resolve_registry("quay.io/lakekeeper/catalog:v1"), "quay.io");
        assert_eq!(
            resolve_registry("registry.example.com/app:1.0"),
            "registry.example.com"
        );
        assert_eq!(
            resolve_registry("my-registry.io:5000/image:tag"),
            "my-registry.io:5000"
        );
    }

    #[test]
    fn test_resolve_registry_with_digest() {
        assert_eq!(
            resolve_registry("postgres@sha256:abcdef1234567890"),
            DOCKER_HUB_REGISTRY
        );
        assert_eq!(resolve_registry("ghcr.io/org/img@sha256:abcdef"), "ghcr.io");
    }

    fn ustar_header(entry_type: EntryType, size: usize) -> tar::Header {
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(entry_type);
        header.set_size(u64::try_from(size).unwrap());
        header.set_mode(0o644);
        header
    }

    /// Encodes one PAX record, whose leading decimal length counts the whole record itself.
    fn pax_record(key: &str, value: &str) -> Vec<u8> {
        let body = format!(" {key}={value}\n");
        let mut len = body.len();
        while len.to_string().len() + body.len() != len {
            len = len.to_string().len() + body.len();
        }
        format!("{len}{body}").into_bytes()
    }

    /// Builds a tar holding a PAX header naming `pax_path`, `payload` as `result.txt`, then
    /// `next.txt`.
    fn archive_with_pax_header(pax_path: &str, payload: &[u8]) -> Vec<u8> {
        let pax_records = pax_record("path", pax_path);
        let mut builder = tar::Builder::new(Vec::new());
        builder
            .append_data(
                &mut ustar_header(EntryType::XHeader, pax_records.len()),
                "PaxHeaders/result.txt",
                pax_records.as_slice(),
            )
            .unwrap();
        builder
            .append_data(
                &mut ustar_header(EntryType::Regular, payload.len()),
                "result.txt",
                payload,
            )
            .unwrap();
        builder
            .append_data(
                &mut ustar_header(EntryType::Regular, 5),
                "next.txt",
                &b"other"[..],
            )
            .unwrap();
        builder.into_inner().unwrap()
    }

    #[tokio::test]
    async fn extract_file_entry_streams_only_the_payload_after_a_long_pax_header() {
        let payload: Vec<u8> = (0..20_000u32)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        let pax_path = format!("pax/{}/result.txt", "d".repeat(50_000));
        let archive = archive_with_pax_header(&pax_path, &payload);
        let (reader, mut writer) = tokio::io::duplex(100);
        let feeder = tokio::spawn(async move {
            // The reader stops at the end of `result.txt`, so the rest may hit a closed pipe.
            let _ = writer.write_all(&archive).await;
        });

        let copied = Client::extract_file_entry(reader, Vec::new())
            .await
            .unwrap();
        feeder.await.unwrap();

        assert_eq!(copied, payload);
    }

    #[tokio::test]
    async fn extract_file_entry_rejects_a_truncated_payload() {
        let mut archive = archive_with_pax_header("pax/result.txt", b"payload");
        archive.truncate(3 * 512 + 3);

        let err = Client::extract_file_entry(std::io::Cursor::new(archive), Vec::new())
            .await
            .unwrap_err();

        assert!(matches!(
            &err,
            CopyFromContainerError::Io(e) if e.kind() == io::ErrorKind::UnexpectedEof
        ));
    }

    #[tokio::test]
    async fn extract_file_entry_releases_the_reader_when_cancelled() {
        let (reader, mut writer) = tokio::io::duplex(1024);

        tokio::select! {
            biased;
            _ = Client::extract_file_entry(reader, Vec::new()) => {
                panic!("extraction finished without any archive bytes");
            }
            () = std::future::ready(()) => {}
        }

        let err = writer.write_all(b"late").await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    #[tokio::test]
    async fn extract_file_entry_rejects_a_symlink() {
        let mut builder = tar::Builder::new(Vec::new());
        builder
            .append_link(
                &mut ustar_header(EntryType::Symlink, 0),
                "link",
                "result.txt",
            )
            .unwrap();
        let archive = std::io::Cursor::new(builder.into_inner().unwrap());

        let err = Client::extract_file_entry(archive, Vec::new())
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            CopyFromContainerError::UnsupportedEntry(EntryType::Symlink)
        ));
    }

    #[tokio::test]
    async fn extract_file_entry_rejects_an_archive_without_entries() {
        let archive = std::io::Cursor::new(tar::Builder::new(Vec::new()).into_inner().unwrap());

        let err = Client::extract_file_entry(archive, Vec::new())
            .await
            .unwrap_err();

        assert!(matches!(err, CopyFromContainerError::EmptyArchive));
    }
}
