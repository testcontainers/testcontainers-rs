use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};

use crate::{
    compose::client::ComposeInterface,
    core::{
        async_container::raw::RawContainer, async_drop, client::Client, env, wait::WaitStrategy,
        WaitFor,
    },
};

mod client;
mod error;

pub use error::{ComposeError, Result};

const COMPOSE_PROJECT_LABEL: &str = "com.docker.compose.project";
const COMPOSE_SERVICE_LABEL: &str = "com.docker.compose.service";

/// Configuration for the local docker compose client.
#[derive(Clone, Debug)]
pub struct LocalComposeOptions {
    compose_files: Vec<PathBuf>,
}

impl LocalComposeOptions {
    pub fn new(compose_files: &[impl AsRef<Path>]) -> Self {
        let compose_files = compose_files
            .iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        Self { compose_files }
    }

    pub(crate) fn into_parts(self) -> Vec<PathBuf> {
        self.compose_files
    }
}

/// Configuration for the containerised docker compose client.
///
/// The project directory is used by docker compose to resolve relative paths in compose files.
/// For host-relative bind mounts, pass an absolute host path.
#[derive(Clone, Debug)]
pub struct ContainerisedComposeOptions {
    compose_files: Vec<PathBuf>,
    project_directory: Option<PathBuf>,
}

impl ContainerisedComposeOptions {
    pub fn new(compose_files: &[impl AsRef<Path>]) -> Self {
        let compose_files = compose_files
            .iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        Self {
            compose_files,
            project_directory: None,
        }
    }

    pub fn with_project_directory(mut self, project_directory: impl AsRef<Path>) -> Self {
        self.project_directory = Some(project_directory.as_ref().to_path_buf());
        self
    }

    pub(crate) fn into_parts(self) -> (Vec<PathBuf>, Option<PathBuf>) {
        (self.compose_files, self.project_directory)
    }
}

/// Configuration for automatic docker compose client selection.
#[derive(Clone, Debug)]
pub struct AutoComposeOptions {
    local: LocalComposeOptions,
    containerised: ContainerisedComposeOptions,
}

impl AutoComposeOptions {
    pub fn new(compose_files: &[impl AsRef<Path>]) -> Self {
        let local = LocalComposeOptions::new(compose_files);
        let containerised = ContainerisedComposeOptions::new(compose_files);

        Self {
            local,
            containerised,
        }
    }

    pub fn from_local(local: LocalComposeOptions) -> Self {
        let containerised = ContainerisedComposeOptions {
            compose_files: local.compose_files.clone(),
            project_directory: None,
        };

        Self {
            local,
            containerised,
        }
    }

    pub fn from_containerised(containerised: ContainerisedComposeOptions) -> Self {
        let local = LocalComposeOptions {
            compose_files: containerised.compose_files.clone(),
        };

        Self {
            local,
            containerised,
        }
    }

    pub fn with_local_options(mut self, local: LocalComposeOptions) -> Self {
        self.local = local;
        self
    }

    pub fn with_containerised_options(
        mut self,
        containerised: ContainerisedComposeOptions,
    ) -> Self {
        self.containerised = containerised;
        self
    }

    pub(crate) fn into_parts(self) -> (LocalComposeOptions, ContainerisedComposeOptions) {
        (self.local, self.containerised)
    }
}

pub struct DockerCompose {
    project_name: String,
    client: Arc<client::ComposeClient>,
    docker_client: Option<Arc<Client>>,
    remove_volumes: bool,
    remove_images: bool,
    build: bool,
    pull: bool,
    services: HashMap<String, RawContainer>,
    env_vars: HashMap<String, String>,
    wait_strategies: HashMap<String, WaitFor>,
    dropped: bool,
}

impl std::fmt::Debug for DockerCompose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockerCompose")
            .field("project_name", &self.project_name)
            .field("client", &self.client)
            .field("remove_volumes", &self.remove_volumes)
            .field("remove_images", &self.remove_images)
            .field("services", &self.services.keys())
            .field("env_vars", &self.env_vars)
            .field("wait_strategies", &self.wait_strategies.keys())
            .finish()
    }
}

impl DockerCompose {
    /// Create a new docker compose with a local client (using docker-cli installed locally).
    /// If you don't have docker-cli installed, you can use `with_containerised_client` instead.
    ///
    /// Accepts any iterable of paths (slices, arrays, vecs, iterators):
    /// ```rust,no_run
    /// use testcontainers::compose::DockerCompose;
    ///
    /// let compose = DockerCompose::with_local_client(&["docker-compose.yml"]);
    /// let compose = DockerCompose::with_local_client(vec!["docker-compose.yml"]);
    /// let compose = DockerCompose::with_local_client(std::iter::once("docker-compose.yml"));
    /// ```
    pub fn with_local_client(options: impl Into<LocalComposeOptions>) -> Self {
        let options = options.into();
        let client = Arc::new(client::ComposeClient::new_local(options.into_parts()));

        Self::new(client)
    }

    /// Create a new docker compose with a containerised client (doesn't require docker-cli installed locally).
    ///
    /// Pass a slice of compose files for the default behavior, or provide
    /// [`ContainerisedComposeOptions`] to set the project directory.
    ///
    /// ```rust,no_run
    /// use testcontainers::compose::{ContainerisedComposeOptions, DockerCompose};
    ///
    /// let compose = DockerCompose::with_containerised_client(&["docker-compose.yml"]).await?;
    ///
    /// let options = ContainerisedComposeOptions::new(&["/home/me/app/docker-compose.yml"])
    ///     .with_project_directory("/home/me/app");
    /// let compose = DockerCompose::with_containerised_client(options).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn with_containerised_client(
        options: impl Into<ContainerisedComposeOptions>,
    ) -> Result<Self> {
        let options = options.into();
        let client = Arc::new(client::ComposeClient::new_containerised(options).await?);

        Ok(Self::new(client))
    }

    /// Create a new docker compose with automatic client selection.
    ///
    /// The local client is selected when `docker compose` is available, otherwise the
    /// containerised client is used.
    ///
    /// ```rust,no_run
    /// use testcontainers::compose::{AutoComposeOptions, ContainerisedComposeOptions, DockerCompose};
    ///
    /// let compose = DockerCompose::with_auto_client(&["docker-compose.yml"]).await?;
    ///
    /// let containerised = ContainerisedComposeOptions::new(&["docker-compose.yml"])
    ///     .with_project_directory("/home/me/app");
    /// let auto = AutoComposeOptions::from_containerised(containerised);
    /// let compose = DockerCompose::with_auto_client(auto).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn with_auto_client(options: impl Into<AutoComposeOptions>) -> Result<Self> {
        let (local, containerised) = options.into().into_parts();
        if local_compose_available().await {
            let client = Arc::new(client::ComposeClient::new_local(local.into_parts()));
            Ok(Self::new(client))
        } else {
            let client = Arc::new(client::ComposeClient::new_containerised(containerised).await?);
            Ok(Self::new(client))
        }
    }

    /// Start the docker compose and discover all services
    pub async fn up(&mut self) -> Result<()> {
        self.client
            .up(client::UpCommand {
                project_name: self.project_name.clone(),
                wait_timeout: std::time::Duration::from_secs(60),
                env_vars: self.env_vars.clone(),
                build: self.build,
                pull: self.pull,
            })
            .await?;

        let docker_client = Client::lazy_client().await?;

        let containers = docker_client
            .list_containers_by_label(COMPOSE_PROJECT_LABEL, &self.project_name)
            .await?;

        for container in containers {
            if let (Some(labels), Some(id)) = (container.labels, container.id) {
                if let Some(service_name) = labels.get(COMPOSE_SERVICE_LABEL) {
                    let raw = RawContainer::new(id, docker_client.clone());
                    self.services.insert(service_name.clone(), raw);
                }
            }
        }

        self.docker_client = Some(docker_client.clone());

        for (service_name, wait_strategy) in &self.wait_strategies {
            let container = self
                .service(service_name)
                .ok_or_else(|| ComposeError::ServiceNotFound(service_name.clone()))?;

            wait_strategy
                .clone()
                .wait_until_ready(&docker_client, container)
                .await?;
        }

        Ok(())
    }

    /// Get a reference to a service container
    pub fn service(&self, name: &str) -> Option<&RawContainer> {
        self.services.get(name)
    }

    /// List all discovered service names
    pub fn services(&self) -> Vec<&str> {
        self.services.keys().map(|s| s.as_str()).collect()
    }

    /// Set environment variable for docker compose
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables for docker compose
    pub fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Add a wait strategy for a specific service
    pub fn with_wait_for_service(mut self, service_name: impl Into<String>, wait: WaitFor) -> Self {
        self.wait_strategies.insert(service_name.into(), wait);
        self
    }

    /// Override the docker compose project name (default: random UUID)
    pub fn with_project_name(mut self, project_name: impl Into<String>) -> Self {
        self.project_name = project_name.into();
        self
    }

    /// Explicitly stop and remove the compose stack
    ///
    /// Consumes the DockerCompose instance since the stack is no longer usable after teardown.
    /// If not called, the stack will be automatically cleaned up when DockerCompose is dropped.
    pub async fn down(mut self) -> Result<()> {
        self.stopped_explicitly().await?;
        Ok(())
    }

    async fn stopped_explicitly(&mut self) -> Result<()> {
        if self.dropped {
            return Ok(());
        }

        self.client
            .down(client::DownCommand {
                project_name: self.project_name.clone(),
                rmi: self.remove_images,
                volumes: self.remove_volumes,
            })
            .await?;

        self.services.clear();
        self.docker_client = None;
        self.dropped = true;

        Ok(())
    }

    /// Build images before starting services (default: false)
    pub fn with_build(mut self, build: bool) -> Self {
        self.build = build;
        self
    }

    /// Pull images before starting services (default: false)
    pub fn with_pull(mut self, pull: bool) -> Self {
        self.pull = pull;
        self
    }

    /// Remove volumes when dropping the docker compose or not (removed by default)
    pub fn with_remove_volumes(&mut self, remove_volumes: bool) -> &mut Self {
        self.remove_volumes = remove_volumes;
        self
    }

    /// Remove images when dropping the docker compose or not
    pub fn with_remove_images(&mut self, remove_images: bool) -> &mut Self {
        self.remove_images = remove_images;
        self
    }

    fn new(client: Arc<client::ComposeClient>) -> Self {
        let project_name = uuid::Uuid::new_v4().to_string();

        Self {
            project_name,
            client,
            docker_client: None,
            remove_volumes: true,
            remove_images: false,
            build: false,
            pull: false,
            services: HashMap::new(),
            env_vars: HashMap::new(),
            wait_strategies: HashMap::new(),
            dropped: false,
        }
    }
}

impl Drop for DockerCompose {
    fn drop(&mut self) {
        if self.dropped {
            return;
        }

        let project_name = self.project_name.clone();
        let client = self.client.clone();
        let rmi = self.remove_images;
        let volumes = self.remove_volumes;
        let command = self
            .docker_client
            .as_ref()
            .map(|client| client.config.command())
            .unwrap_or(env::Command::Remove);

        let drop_task = async move {
            if command != env::Command::Remove {
                return;
            }

            let res = client
                .down(client::DownCommand {
                    project_name,
                    rmi,
                    volumes,
                })
                .await;

            match res {
                Ok(()) => log::info!("docker compose successfully dropped"),
                Err(e) => log::error!("failed to drop docker compose: {}", e),
            }
        };

        async_drop::async_drop(drop_task);
    }
}

async fn local_compose_available() -> bool {
    let output = tokio::process::Command::new("docker")
        .arg("compose")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;

    matches!(output, Ok(output) if output.status.success())
}

impl<T> From<&[T]> for LocalComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: &[T]) -> Self {
        Self::new(value)
    }
}

impl<T> From<Vec<T>> for LocalComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: Vec<T>) -> Self {
        let compose_files = value
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        Self { compose_files }
    }
}

impl<T, const N: usize> From<&[T; N]> for LocalComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: &[T; N]) -> Self {
        Self::from(value.as_slice())
    }
}

impl From<&LocalComposeOptions> for LocalComposeOptions {
    fn from(value: &LocalComposeOptions) -> Self {
        value.clone()
    }
}

impl<T> From<&[T]> for ContainerisedComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: &[T]) -> Self {
        Self::new(value)
    }
}

impl<T> From<Vec<T>> for ContainerisedComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: Vec<T>) -> Self {
        let compose_files = value
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        Self {
            compose_files,
            project_directory: None,
        }
    }
}

impl<T, const N: usize> From<&[T; N]> for ContainerisedComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: &[T; N]) -> Self {
        Self::from(value.as_slice())
    }
}

impl From<&ContainerisedComposeOptions> for ContainerisedComposeOptions {
    fn from(value: &ContainerisedComposeOptions) -> Self {
        value.clone()
    }
}

impl<T> From<&[T]> for AutoComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: &[T]) -> Self {
        Self::new(value)
    }
}

impl<T> From<Vec<T>> for AutoComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: Vec<T>) -> Self {
        let compose_files = value
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect::<Vec<_>>();

        let local = LocalComposeOptions {
            compose_files: compose_files.clone(),
        };
        let containerised = ContainerisedComposeOptions {
            compose_files,
            project_directory: None,
        };

        Self {
            local,
            containerised,
        }
    }
}

impl<T, const N: usize> From<&[T; N]> for AutoComposeOptions
where
    T: AsRef<Path>,
{
    fn from(value: &[T; N]) -> Self {
        Self::from(value.as_slice())
    }
}

impl From<&AutoComposeOptions> for AutoComposeOptions {
    fn from(value: &AutoComposeOptions) -> Self {
        value.clone()
    }
}

impl From<LocalComposeOptions> for AutoComposeOptions {
    fn from(value: LocalComposeOptions) -> Self {
        AutoComposeOptions::from_local(value)
    }
}

impl From<&LocalComposeOptions> for AutoComposeOptions {
    fn from(value: &LocalComposeOptions) -> Self {
        AutoComposeOptions::from_local(value.clone())
    }
}

impl From<ContainerisedComposeOptions> for AutoComposeOptions {
    fn from(value: ContainerisedComposeOptions) -> Self {
        AutoComposeOptions::from_containerised(value)
    }
}

impl From<&ContainerisedComposeOptions> for AutoComposeOptions {
    fn from(value: &ContainerisedComposeOptions) -> Self {
        AutoComposeOptions::from_containerised(value.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn test_local_docker_compose() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let path_to_compose = PathBuf::from(format!(
            "{}/tests/test-compose.yml",
            env!("CARGO_MANIFEST_DIR")
        ));

        let mut compose = DockerCompose::with_local_client(&[path_to_compose.as_path()]);

        compose.up().await?;

        println!("Services: {:?}", compose.services());

        let hello1 = compose
            .service("hello1")
            .expect("hello1 service should exist");
        let hello2 = compose
            .service("hello2")
            .expect("hello2 service should exist");

        let port1 = hello1.get_host_port_ipv4(8080).await?;
        let port2 = hello2.get_host_port_ipv4(8080).await?;
        println!("Services on ports: {} and {}", port1, port2);

        let response = reqwest::get(format!("http://localhost:{}", port1))
            .await?
            .status();

        assert!(response.is_success(), "Service should respond");

        Ok(())
    }

    #[tokio::test]
    async fn test_compose_with_build_and_down() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let path_to_compose = PathBuf::from(format!(
            "{}/tests/test-compose.yml",
            env!("CARGO_MANIFEST_DIR")
        ));

        let mut compose = DockerCompose::with_local_client(&[path_to_compose.as_path()])
            .with_env("TEST_VAR", "test_value");

        compose.up().await?;

        println!("Services discovered: {:?}", compose.services());
        assert_eq!(compose.services().len(), 2, "Should have 2 services");

        let hello1 = compose.service("hello1").expect("hello1 should exist");
        assert!(!hello1.id().is_empty(), "Container ID should be set");

        compose.down().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_compose_exec_and_logs() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let path_to_compose = PathBuf::from(format!(
            "{}/tests/test-compose.yml",
            env!("CARGO_MANIFEST_DIR")
        ));

        let mut compose = DockerCompose::with_local_client(&[path_to_compose.as_path()]);
        compose.up().await?;

        let hello1 = compose.service("hello1").expect("hello1 should exist");
        let hello2 = compose.service("hello2").expect("hello2 should exist");

        let container_id = hello1.id();
        assert!(!container_id.is_empty(), "Container ID should not be empty");

        let port1 = hello1.get_host_port_ipv4(8080).await?;
        let port2 = hello2.get_host_port_ipv4(8080).await?;

        assert_ne!(port1, port2, "Services should have different host ports");

        let response1 = reqwest::get(format!("http://localhost:{}", port1)).await?;
        let response2 = reqwest::get(format!("http://localhost:{}", port2)).await?;

        assert!(response1.status().is_success());
        assert!(response2.status().is_success());

        Ok(())
    }

    async fn test_compose_client(mut compose: DockerCompose, mode: &str) -> anyhow::Result<()> {
        compose.up().await?;

        assert_eq!(
            compose.services().len(),
            2,
            "{} mode: should have 2 services",
            mode
        );

        let hello1 = compose
            .service("hello1")
            .unwrap_or_else(|| panic!("{} mode: hello1 service should exist", mode));
        let hello2 = compose
            .service("hello2")
            .unwrap_or_else(|| panic!("{} mode: hello2 service should exist", mode));

        let port1 = hello1.get_host_port_ipv4(8080).await?;
        let port2 = hello2.get_host_port_ipv4(8080).await?;

        println!("{} mode: hello1 on {}, hello2 on {}", mode, port1, port2);

        let response = reqwest::get(format!("http://localhost:{}", port1))
            .await?
            .status();

        assert!(
            response.is_success(),
            "{} mode: service should respond",
            mode
        );

        compose.down().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_local_client_mode() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let path = PathBuf::from(format!(
            "{}/tests/test-compose.yml",
            env!("CARGO_MANIFEST_DIR")
        ));
        let compose = DockerCompose::with_local_client(&[path.as_path()]);

        test_compose_client(compose, "local").await
    }

    #[tokio::test]
    async fn test_containerised_client_mode() -> anyhow::Result<()> {
        let _ = pretty_env_logger::try_init();

        let path = PathBuf::from(format!(
            "{}/tests/test-compose.yml",
            env!("CARGO_MANIFEST_DIR")
        ));
        let compose = DockerCompose::with_containerised_client(&[path.as_path()]).await?;

        test_compose_client(compose, "containerised").await
    }
}
