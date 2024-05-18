use std::{io, sync::Arc, time::Duration};

use bollard::{
    auth::DockerCredentials,
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions},
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    image::CreateImageOptions,
    network::{CreateNetworkOptions, InspectNetworkOptions},
    Docker,
};
use bollard_stubs::models::{
    ContainerCreateResponse, ContainerInspectResponse, ExecInspectResponse, HealthStatusEnum,
    Network,
};
use futures::{StreamExt, TryStreamExt};
use tokio::sync::OnceCell;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::core::{
    client::exec::ExecResult,
    env,
    errors::WaitContainerError,
    logs::{LogSource, LogStreamAsync},
    ports::Ports,
    WaitFor,
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

/// The internal client.
pub(crate) struct Client {
    pub(crate) config: env::Config,
    pub(crate) bollard: Docker,
}

impl Client {
    async fn new() -> Client {
        let config = env::Config::load::<env::Os>().await;
        let bollard = bollard_client::init(&config);

        Client { config, bollard }
    }

    pub(crate) fn stdout_logs(&self, id: &str) -> LogStreamAsync<'_> {
        self.logs(id, LogSource::StdOut)
    }

    pub(crate) fn stderr_logs(&self, id: &str) -> LogStreamAsync<'_> {
        self.logs(id, LogSource::StdErr)
    }

    pub(crate) async fn ports(&self, id: &str) -> Ports {
        self.inspect(id)
            .await
            .unwrap()
            .network_settings
            .unwrap_or_default()
            .ports
            .map(Ports::from)
            .unwrap_or_default()
    }

    pub(crate) async fn inspect(
        &self,
        id: &str,
    ) -> Result<ContainerInspectResponse, bollard::errors::Error> {
        self.bollard.inspect_container(id, None).await
    }

    pub(crate) async fn rm(&self, id: &str) {
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

    pub(crate) async fn stop(&self, id: &str) {
        self.bollard.stop_container(id, None).await.unwrap();
    }

    pub(crate) async fn start(&self, id: &str) {
        self.bollard
            .start_container::<String>(id, None)
            .await
            .unwrap();
    }

    pub(crate) async fn exec(
        &self,
        container_id: &str,
        cmd: Vec<String>,
    ) -> Result<ExecResult<'_>, bollard::errors::Error> {
        let config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self.bollard.create_exec(container_id, config).await?;

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
            .await?;

        match res {
            StartExecResults::Attached { output, .. } => {
                let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel();
                let (stderr_tx, stderr_rx) = tokio::sync::mpsc::unbounded_channel();
                let stdout_notifier = Arc::new(tokio::sync::Semaphore::new(100));
                let stderr_notifier = stdout_notifier.clone();

                let sender_backpressure = stdout_notifier.clone();
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
                        // don't produce extra messages until the receiver is ready to consume them
                        handle_error!(sender_backpressure.acquire().await);
                        sender_backpressure.forget_permits(1);
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

                let stdout = LogStreamAsync::new(
                    UnboundedReceiverStream::new(stdout_rx)
                        .inspect(move |_| stdout_notifier.add_permits(1))
                        .boxed(),
                )
                .enable_cache();
                let stderr = LogStreamAsync::new(
                    UnboundedReceiverStream::new(stderr_rx)
                        .inspect(move |_| stderr_notifier.add_permits(1))
                        .boxed(),
                )
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
    ) -> Result<ExecInspectResponse, bollard::errors::Error> {
        self.bollard.inspect_exec(exec_id).await
    }

    pub(crate) async fn block_until_ready(
        &self,
        id: &str,
        ready_conditions: &[WaitFor],
    ) -> Result<(), WaitContainerError> {
        log::debug!("Waiting for container {id} to be ready");

        for condition in ready_conditions {
            match condition {
                WaitFor::StdOutMessage { message } => {
                    self.stdout_logs(id).wait_for_message(message).await?
                }
                WaitFor::StdErrMessage { message } => {
                    self.stderr_logs(id).wait_for_message(message).await?
                }
                WaitFor::Duration { length } => {
                    tokio::time::sleep(*length).await;
                }
                WaitFor::Healthcheck => loop {
                    use HealthStatusEnum::*;

                    let health_status = self
                        .inspect(id)
                        .await
                        .map_err(WaitContainerError::Inspect)?
                        .state
                        .ok_or(WaitContainerError::StateUnavailable)?
                        .health
                        .and_then(|health| health.status);

                    match health_status {
                        Some(HEALTHY) => break,
                        None | Some(EMPTY) | Some(NONE) => {
                            return Err(WaitContainerError::HealthCheckNotConfigured(
                                id.to_string(),
                            ))
                        }
                        Some(UNHEALTHY) => return Err(WaitContainerError::Unhealthy),
                        Some(STARTING) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                },
                WaitFor::Nothing => {}
            }
        }

        log::debug!("Container {id} is now ready!");
        Ok(())
    }

    fn logs(&self, container_id: &str, log_source: LogSource) -> LogStreamAsync<'_> {
        let options = LogsOptions {
            follow: true,
            stdout: log_source.is_stdout(),
            stderr: log_source.is_stderr(),
            tail: "all".to_owned(),
            ..Default::default()
        };

        let stream = self
            .bollard
            .logs(container_id, Some(options))
            .map_ok(|chunk| chunk.into_bytes())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
            .boxed();
        LogStreamAsync::new(stream)
    }

    /// Creates a network with given name and returns an ID
    pub(crate) async fn create_network(&self, name: &str) -> Option<String> {
        let network = self
            .bollard
            .create_network(CreateNetworkOptions {
                name: name.to_owned(),
                check_duplicate: true,
                ..Default::default()
            })
            .await
            .unwrap();

        network.id
    }

    /// Inspects a network
    pub(crate) async fn inspect_network(
        &self,
        name: &str,
    ) -> Result<Network, bollard::errors::Error> {
        self.bollard
            .inspect_network(name, Some(InspectNetworkOptions::<String>::default()))
            .await
    }

    pub(crate) async fn create_container(
        &self,
        options: Option<CreateContainerOptions<String>>,
        config: Config<String>,
    ) -> Result<ContainerCreateResponse, bollard::errors::Error> {
        self.bollard.create_container(options, config).await
    }

    pub(crate) async fn pull_image(&self, descriptor: &str) {
        let pull_options = Some(CreateImageOptions {
            from_image: descriptor,
            ..Default::default()
        });
        let credentials = self.credentials_for_image(descriptor).await;
        let mut pulling = self.bollard.create_image(pull_options, None, credentials);
        while let Some(result) = pulling.next().await {
            result.unwrap_or_else(|err| {
                panic!("Error pulling the image: '{descriptor}', error: {err}")
            });
        }
    }

    pub(crate) async fn network_exists(&self, network: &str) -> bool {
        let networks = self.bollard.list_networks::<String>(None).await.unwrap();
        networks
            .iter()
            .any(|i| matches!(&i.name, Some(name) if name == network))
    }

    pub(crate) async fn remove_network(&self, network: &str) {
        self.bollard
            .remove_network(network)
            .await
            .expect("Failed to remove network");
    }

    pub(crate) async fn docker_hostname(&self) -> url::Host {
        let docker_host = self.config.docker_host();
        match docker_host.scheme() {
            "tcp" | "http" | "https" => docker_host.host().unwrap().to_owned(),
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
                        .unwrap_or_else(|e| panic!("invalid host: '{host}', error: {e}"))
                } else {
                    url::Host::Domain("localhost".to_string())
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
        .unwrap()?;

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
