use std::{io, time::Duration};

use bollard::{
    auth::DockerCredentials,
    container::{Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions},
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
    image::CreateImageOptions,
    network::CreateNetworkOptions,
    Docker,
};
use bollard_stubs::models::{ContainerCreateResponse, ContainerInspectResponse, HealthStatusEnum};
use futures::{StreamExt, TryStreamExt};

use crate::core::{env, logs::LogStreamAsync, ports::Ports, WaitFor};

mod bollard_client;
mod factory;

/// The desired log stream.
pub(crate) enum DesiredLogStream {
    Stdout,
    Stderr,
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
        self.logs(id, DesiredLogStream::Stdout)
    }

    pub(crate) fn stderr_logs(&self, id: &str) -> LogStreamAsync<'_> {
        self.logs(id, DesiredLogStream::Stderr)
    }

    pub(crate) async fn ports(&self, id: &str) -> Ports {
        self.inspect(id)
            .await
            .network_settings
            .unwrap_or_default()
            .ports
            .map(Ports::from)
            .unwrap_or_default()
    }

    pub(crate) async fn inspect(&self, id: &str) -> ContainerInspectResponse {
        self.bollard.inspect_container(id, None).await.unwrap()
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
        attach_stdout: Option<bool>,
        attach_stderr: Option<bool>,
    ) -> LogStreamAsync<'_> {
        let config = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout,
            attach_stderr,
            ..Default::default()
        };

        let exec = self
            .bollard
            .create_exec(container_id, config)
            .await
            .expect("failed to create exec");

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
            .expect("failed to start exec");

        match res {
            StartExecResults::Attached { output, .. } => {
                let stream = output
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
                    .map(|chunk| {
                        let bytes = chunk?.into_bytes();
                        let str = std::str::from_utf8(bytes.as_ref())
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                        Ok(str.to_string())
                    })
                    .boxed();

                LogStreamAsync::new(stream)
            }
            StartExecResults::Detached => unreachable!("detach is false"),
        }
    }

    pub(crate) async fn block_until_ready(&self, id: &str, ready_conditions: &[WaitFor]) {
        log::debug!("Waiting for container {id} to be ready");

        for condition in ready_conditions {
            match condition {
                WaitFor::StdOutMessage { message } => self
                    .stdout_logs(id)
                    .wait_for_message(message)
                    .await
                    .unwrap(),
                WaitFor::StdErrMessage { message } => self
                    .stderr_logs(id)
                    .wait_for_message(message)
                    .await
                    .unwrap(),
                WaitFor::Duration { length } => {
                    tokio::time::sleep(*length).await;
                }
                WaitFor::Healthcheck => loop {
                    use HealthStatusEnum::*;

                    let health_status = self
                        .inspect(id)
                        .await
                        .state
                        .unwrap_or_else(|| panic!("Container state not available"))
                        .health
                        .unwrap_or_else(|| panic!("Health state not available"))
                        .status;

                    match health_status {
                        Some(HEALTHY) => break,
                        None | Some(EMPTY) | Some(NONE) => {
                            panic!("Healthcheck not configured for container")
                        }
                        Some(UNHEALTHY) => panic!("Healthcheck reports unhealthy"),
                        Some(STARTING) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                },
                WaitFor::Nothing => {}
            }
        }

        log::debug!("Container {id} is now ready!");
    }

    fn logs(&self, container_id: &str, desired_log: DesiredLogStream) -> LogStreamAsync<'_> {
        let (stdout, stderr) = match desired_log {
            DesiredLogStream::Stdout => (true, false),
            DesiredLogStream::Stderr => (false, true),
        };
        let options = LogsOptions {
            follow: true,
            stdout,
            stderr,
            tail: "all".to_owned(),
            ..Default::default()
        };

        let stream = self
            .bollard
            .logs(container_id, Some(options))
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
        let host = match docker_host.scheme() {
            "tcp" | "http" | "https" => docker_host.host().unwrap().to_string(),
            "unix" | "npipe" => self
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
                .unwrap_or_else(|| "localhost".to_string()),
            _ => unreachable!("docker host is already validated in the config"),
        };

        url::Host::parse(&host).unwrap_or_else(|e| panic!("invalid host: '{host}', error: {e}"))
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
