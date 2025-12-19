use std::{collections::HashMap, path::Path, sync::Arc};

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
    /// Create a new docker compose with a local client (using docker-cli installed locally)
    /// If you don't have docker-cli installed, you can use `with_containerised_client` instead
    pub fn with_local_client(compose_files: &[impl AsRef<Path>]) -> Self {
        let compose_files = compose_files
            .iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        let client = Arc::new(client::ComposeClient::new_local(compose_files));

        Self::new(client)
    }

    /// Create a new docker compose with a containerised client (doesn't require docker-cli installed locally)
    pub async fn with_containerised_client(compose_files: &[impl AsRef<Path>]) -> Result<Self> {
        let compose_files = compose_files
            .iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        let client = Arc::new(client::ComposeClient::new_containerised(compose_files).await?);

        Ok(Self::new(client))
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
