//! Testing with Docker compose
//!
//! Requirements:
//!
//! * enabling the `compose` feature.
//! * have a `docker compose` v2.x.x installed
//!
//! ```
//! let dc = DockerCompose::builder()
//!     .env("MY_ENV_VAR", "MyValue")
//!     .wait("mydb", WaitFor::message_on_stdout("database is ready"))
//!     .wait("webserver", WaitFor::Healthcheck)
//!     .build();
//! dc.up();
//! dc.block_until_ready();
//!
//! let port = dc.get_mapped_port("webserver", 80).unwrap();
//! // testing my web server...
//! ```
//! You can configure the behavior when the `DockerCompose` is dropped
//! by choosing a [`StopMode`](crate::compose::StopMode) with [`DockerComposeBuilder::stop_with`](crate::compose::DockerComposeBuilder::stop_with)
//!
//! For debugging purpose, you can show `docker compose` output with [`DockerComposeBuilder::inherit_io`](crate::compose::DockerComposeBuilder::inherit_io)
//!
//! You can found more examples in the `tests/compose.rs` file
//!

use std::{
    collections::HashMap,
    io::{BufRead, Read},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread::sleep,
    time::{Duration, SystemTime},
};

use bollard_stubs::models::{ContainerStateStatusEnum, HealthStatusEnum};
use log::{debug, info, trace};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::core::{
    logs::{LogStream, WaitError},
    WaitFor,
};

mod builder;
pub use self::builder::*;

/// Define the behavior of the [`DockerCompose`] drop
/// By default we use [`StopMode::Stop`]
#[derive(Debug, Clone, Copy)]
pub enum StopMode {
    /// Stop containers
    Stop,
    /// Stop and remove containers
    StopAndRemove,
    /// Do nothing, containers still running after the drop
    Detach,
}

impl Default for StopMode {
    fn default() -> Self {
        Self::Stop
    }
}

/// Implementation of the Docker compose client using the CLI

#[derive(Debug)]
pub struct DockerCompose {
    path: PathBuf,
    env: HashMap<String, String>,
    child: Option<Child>,
    ready_conditions: Vec<(String, WaitFor)>,
    stop_mode: StopMode,
    inherit_io: bool,
}

// Trick to avoid code to be run concurrently
static NO_PARALLEL: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

impl DockerCompose {
    /// Create the builder
    pub fn builder(path: impl AsRef<Path>) -> DockerComposeBuilder {
        DockerComposeBuilder::new(path)
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::new("docker");
        cmd.arg("compose");
        cmd.arg("--file");
        cmd.arg(&self.path);
        cmd.envs(&self.env);

        cmd
    }

    fn pull(&self) -> Result<(), std::io::Error> {
        let mut cmd = self.cmd();
        cmd.args(["pull"]);
        trace!("Starting command {:?}", cmd);
        let _ = cmd.spawn()?.wait()?;
        Ok(())
    }

    /// Call the `up` command
    pub fn up(&mut self) {
        info!("Starting {:?}", self.path);
        // Could fail when launched in parallel with same docker-compose file
        // So we use a locking guard to avoid that situation
        let _shared = NO_PARALLEL.lock();

        // Pulling with inherited stdout/stderr
        self.pull().expect("Failed to execute docker compose pull");

        let mut cmd = self.cmd();
        cmd.args(["up"]);
        trace!("Starting command {:?}", cmd);
        if !self.inherit_io {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }
        let child = cmd
            .spawn()
            .expect("Failed to execute docker compose up command");
        self.child = Some(child);
    }

    /// Wait until each [`WaitFor`] for service are ready
    pub fn block_until_ready(&self) {
        info!("Waiting ready {:?}", self.path);
        let start = SystemTime::now();
        for (service, wait) in self.ready_conditions.iter() {
            debug!("Waiting service {} and {:?}", service, wait);
            match wait {
                WaitFor::StdOutMessage { message } => {
                    let child = self.logs(service, Stdio::piped(), Stdio::null());
                    self.wait_log_stream(service, child.stdout.unwrap(), message)
                        .expect("Message not found in stdout");
                }
                WaitFor::StdErrMessage { message } => {
                    let child = self.logs(service, Stdio::null(), Stdio::piped());
                    self.wait_log_stream(service, child.stderr.unwrap(), message)
                        .expect("Message not found in stderr");
                }
                WaitFor::Duration { length } => {
                    std::thread::sleep(*length);
                }
                WaitFor::Healthcheck => {
                    self.wait_health_check(service);
                }
                WaitFor::Nothing => {}
            }
        }
        debug!(
            "Wait until ready took {}s",
            start.elapsed().unwrap().as_secs()
        );
    }

    fn logs(&self, service: &str, stdout: Stdio, stderr: Stdio) -> Child {
        let mut cmd = self.cmd();
        cmd.args(["logs", "--follow", "--no-log-prefix", service]);
        trace!("Logs command {:?}", cmd);
        cmd.stdout(stdout)
            .stderr(stderr)
            .spawn()
            .expect("Failed to execute docker compose down command")
    }

    fn wait_log_stream(
        &self,
        service: &str,
        stdio: impl Read + 'static,
        message: &str,
    ) -> Result<(), WaitError> {
        self.wait_service_running(service);

        let log_stream = LogStream::new(stdio);
        log_stream.wait_for_message(message)
    }

    fn wait_health_check(&self, service: &str) {
        self.wait_service_running(service);

        loop {
            use HealthStatusEnum::*;
            let health_status = self.service_status(service).map(|it| it.health);

            match health_status {
                Some(HEALTHY) => break,
                None | Some(EMPTY) | Some(NONE) => {
                    panic!("Healthcheck not configured for container")
                }
                Some(UNHEALTHY) => panic!("Healthcheck reports unhealthy"),
                Some(STARTING) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    }

    /// Retrieve the status of services
    pub fn status(&self) -> Vec<ServiceState> {
        let mut cmd = self.cmd();
        cmd.args(["ps", "--format", "json"]);
        trace!("Status command {:?}", cmd);
        let out = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("Failed to execute docker compose ps command");
        serde_json::from_slice::<Vec<ServiceState>>(&out.stdout)
            .expect("Failed to parse docker compose ps command result")
    }

    /// Retrieve a service status
    pub fn service_status(&self, service: &str) -> Option<ServiceState> {
        let mut cmd = self.cmd();
        cmd.args(["ps", "--format", "json", service]);
        trace!("Service status command {:?}", cmd);
        let out = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("Failed to execute docker compose ps command");
        if out.stdout.is_empty() {
            return None;
        }
        let statuses = serde_json::from_slice::<Vec<ServiceState>>(&out.stdout)
            .expect("Failed to parse docker compose ps command result");
        statuses
            .into_iter()
            .find(|status| status.service == service)
    }

    fn wait_service_running(&self, service: &str) {
        loop {
            let status = self.service_status(service).map(|it| it.state);
            trace!("Found service '{service}' status {status:?}");
            if status == Some(ContainerStateStatusEnum::RUNNING) {
                break;
            }
            sleep(Duration::from_millis(100));
        }
    }

    /// Get a service mapped port
    pub fn get_mapped_port(&self, service: &str, internal_port: u16) -> Option<u16> {
        let port = format!("{internal_port}");
        let mut cmd = self.cmd();
        cmd.args(["port", service, &port]);
        trace!("Port command {:?}", cmd);
        let output = cmd
            .output()
            .expect("Failed to execute docker compose port command");
        output.stdout.lines().next().map(|line| {
            line.expect("Failed to get line")
                .parse::<SocketAddr>()
                .expect("Failed to build socket address")
                .port()
        })
    }

    /// Stop containers
    pub fn stop(&mut self) {
        info!("Stopping {:?}", self.path);
        let mut cmd = self.cmd();
        cmd.arg("stop");
        trace!("Stop command {:?}", cmd);
        if !self.inherit_io {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }
        cmd.spawn()
            .expect("Failed to execute down docker compose command")
            .wait()
            .expect("Failed to stop containers");
    }

    /// Remove containers
    pub fn rm(&mut self) {
        info!("Removing {:?}", self.path);
        let mut cmd = self.cmd();
        cmd.args(["rm", "--force"]);
        if !self.inherit_io {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }
        cmd.spawn()
            .expect("Failed to execute rm docker compose command")
            .wait()
            .expect("Failed to remove containers");
    }
}

impl Drop for DockerCompose {
    fn drop(&mut self) {
        debug!("Dropping {:?} with {:?}", self.path, self.stop_mode);
        match self.stop_mode {
            StopMode::Stop => {
                self.stop();
                if let Some(mut child) = self.child.take() {
                    let exit = child.wait().expect("command wasn't running");
                    info!("Exited with status {}", exit);
                }
            }
            StopMode::StopAndRemove => {
                self.stop();
                if let Some(mut child) = self.child.take() {
                    let exit = child.wait().expect("command wasn't running");
                    info!("Exited with status {}", exit);
                }
                self.rm();
            }
            StopMode::Detach => {}
        }
    }
}

/// Represent the service state read from `docker compose ps`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceState {
    #[serde(alias = "ID")]
    id: String,
    name: String,
    command: String,
    project: String,
    service: String,
    state: ContainerStateStatusEnum,
    health: HealthStatusEnum,
    exit_code: Option<i32>,
}

#[cfg(test)]
mod test {

    use super::DockerCompose;

    #[test]
    fn docker_compose_should_be_send_and_sync() {
        assert_send_and_sync::<DockerCompose>();
    }

    fn assert_send_and_sync<T: Send + Sync>() {}
}
