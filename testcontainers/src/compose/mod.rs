use std::{path::Path, sync::Arc};

use crate::{compose::client::ComposeInterface, core::async_drop};

mod client;

#[derive(Debug)]
pub struct DockerCompose {
    project_name: String,
    client: Arc<client::ComposeClient>,
    remove_volumes: bool,
    remove_images: bool,
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
    pub async fn with_containerised_client(compose_files: &[impl AsRef<Path>]) -> Self {
        let compose_files = compose_files
            .iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        let client = Arc::new(client::ComposeClient::new_containerised(compose_files).await);

        Self::new(client)
    }

    /// Start the docker compose
    pub async fn up(&self) {
        self.client
            .up(client::UpCommand {
                project_name: self.project_name.clone(),
                wait_timeout: std::time::Duration::from_secs(60),
            })
            .await
            .expect("TODO: error handling");
    }

    /// Remove volumes when dropping the docker compose or not
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
            remove_volumes: true,
            remove_images: false,
        }
    }
}

impl Drop for DockerCompose {
    fn drop(&mut self) {
        let project_name = self.project_name.clone();
        let client = self.client.clone();
        let rmi = self.remove_images;
        let volumes = self.remove_volumes;
        let drop_task = async move {
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

    // #[tokio::test]
    // async fn test_containerised_docker_compose() {
    //     let path_to_compose = PathBuf::from(format!(
    //         "{}/tests/test-compose.yml",
    //         env!("CARGO_MANIFEST_DIR")
    //     ));
    //     let docker_compose =
    //         DockerCompose::with_containerised_client(&[path_to_compose.as_path()]).await;
    //     docker_compose.up().await;
    //     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    //     let res = reqwest::get("http://localhost:8081/").await.unwrap();
    //     assert!(res.status().is_success());
    // }

    #[tokio::test]
    async fn test_local_docker_compose() {
        let path_to_compose = PathBuf::from(format!(
            "{}/tests/test-compose.yml",
            env!("CARGO_MANIFEST_DIR")
        ));
        let docker_compose = DockerCompose::with_local_client(&[path_to_compose.as_path()]);
        docker_compose.up().await;
        let client = reqwest::get("http://localhost:8081").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
    }
}
