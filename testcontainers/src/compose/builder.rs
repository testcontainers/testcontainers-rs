use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::core::WaitFor;

use super::{DockerCompose, StopMode};

/// A builder for [`DockerCompose`]
#[derive(Debug, Clone, Default)]
pub struct DockerComposeBuilder {
    path: PathBuf,
    env: HashMap<String, String>,
    ready_conditions: Vec<(String, WaitFor)>,
    stop_mode: StopMode,
    inherit_io: bool,
}

impl DockerComposeBuilder {
    /// Create a docker compose builder.
    ///
    /// The path should be the docker compose configuration file, usually `docker-compose.yaml`
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Add an environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add some profiles
    pub fn profiles<T>(mut self, profiles: impl IntoIterator<Item = T>) -> Self
    where
        T: ToString,
    {
        let profiles = profiles
            .into_iter()
            .map(|it| it.to_string())
            .collect::<Vec<_>>()
            .join(",");
        self.env.insert("COMPOSE_PROFILES".to_string(), profiles);
        self
    }

    /// Add a [`WaitFor`] for a service
    pub fn wait(mut self, service: impl Into<String>, wait: WaitFor) -> Self {
        self.ready_conditions.push((service.into(), wait));
        self
    }

    /// Set the stop mode, by default it's [`StopMode::Stop`]
    pub fn stop_with(mut self, stop_mode: StopMode) -> Self {
        self.stop_mode = stop_mode;
        self
    }

    /// Show docker compose stdout and stderr
    pub fn inherit_io(mut self) -> Self {
        self.inherit_io = true;
        self
    }

    /// Create the [`DockerCompose`]
    pub fn build(self) -> DockerCompose {
        DockerCompose {
            path: self.path,
            env: self.env,
            child: None,
            ready_conditions: self.ready_conditions,
            stop_mode: self.stop_mode,
            inherit_io: self.inherit_io,
        }
    }
}
