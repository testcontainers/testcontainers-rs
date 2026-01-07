use std::{fmt, path::PathBuf};

use crate::compose::{error::Result, ContainerisedComposeOptions};

pub(super) mod containerised;
pub(super) mod local;

pub(super) enum ComposeClient {
    Local(local::LocalComposeCli),
    Containerised(Box<containerised::ContainerisedComposeCli>),
}

impl ComposeClient {
    pub(super) fn new_local(compose_files: Vec<PathBuf>) -> Self {
        ComposeClient::Local(local::LocalComposeCli::new(compose_files))
    }

    pub(super) async fn new_containerised(options: ContainerisedComposeOptions) -> Result<Self> {
        Ok(ComposeClient::Containerised(Box::new(
            containerised::ContainerisedComposeCli::new(options).await?,
        )))
    }
}

pub(super) struct UpCommand {
    pub(super) project_name: String,
    pub(super) wait_timeout: std::time::Duration,
    pub(super) env_vars: std::collections::HashMap<String, String>,
    pub(super) build: bool,
    pub(super) pull: bool,
}

pub(super) struct DownCommand {
    pub(super) project_name: String,
    pub(super) rmi: bool,
    pub(super) volumes: bool,
}

pub(super) trait ComposeInterface {
    async fn up(&self, command: UpCommand) -> Result<()>;
    async fn down(&self, command: DownCommand) -> Result<()>;
}

impl ComposeInterface for ComposeClient {
    async fn up(&self, command: UpCommand) -> Result<()> {
        match self {
            ComposeClient::Local(client) => client.up(command).await,
            ComposeClient::Containerised(client) => client.up(command).await,
        }
    }

    async fn down(&self, command: DownCommand) -> Result<()> {
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
