use std::{fmt, path::PathBuf};

use crate::core::async_container::raw::RawContainer;

pub(super) mod containerised;
pub(super) mod local;

pub(super) enum ComposeClient {
    Local(local::LocalComposeCli),
    Containerised(containerised::ContainerisedComposeCli),
}

impl ComposeClient {
    pub(super) fn new_local(compose_files: Vec<PathBuf>) -> Self {
        ComposeClient::Local(local::LocalComposeCli::new(compose_files))
    }

    pub(super) async fn new_containerised(compose_files: Vec<PathBuf>) -> Self {
        ComposeClient::Containerised(
            containerised::ContainerisedComposeCli::new(compose_files).await,
        )
    }
}

pub(super) struct UpCommand {
    pub(super) project_name: String,
    pub(super) wait_timeout: std::time::Duration,
}

pub(super) struct DownCommand {
    pub(super) project_name: String,
    pub(super) rmi: bool,
    pub(super) volumes: bool,
}

pub(super) trait ComposeInterface {
    async fn up(&self, command: UpCommand) -> Result<(), std::io::Error>;
    async fn down(&self, command: DownCommand) -> Result<(), std::io::Error>;
}

impl ComposeInterface for ComposeClient {
    async fn up(&self, command: UpCommand) -> Result<(), std::io::Error> {
        match self {
            ComposeClient::Local(client) => client.up(command).await,
            ComposeClient::Containerised(client) => client.up(command).await,
        }
    }

    async fn down(&self, command: DownCommand) -> Result<(), std::io::Error> {
        match self {
            ComposeClient::Local(client) => client.down(command).await,
            ComposeClient::Containerised(client) => client.down(command).await,
        }
    }
}

impl fmt::Debug for ComposeClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComposeClient::Local(_) => write!(f, "LocalComposeCli"),
            ComposeClient::Containerised(_) => write!(f, "ContainerisedComposeCli"),
        }
    }
}
