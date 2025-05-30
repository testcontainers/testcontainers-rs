use std::time::Duration;

use crate::{
    core::{
        client::Client, error::WaitContainerError, wait::WaitStrategy, CmdWaitFor, ExecCommand,
    },
    ContainerAsync, Image,
};

#[derive(Debug, Clone)]
pub struct CommandStrategy {
    poll_interval: Duration,
    command: ExecCommand,
    fail_fast: bool,
}

impl CommandStrategy {
    /// Create a new `CommandStrategy` with default settings.
    pub fn new() -> Self {
        Self {
            command: ExecCommand::default(),
            poll_interval: Duration::from_millis(100),
            fail_fast: false,
        }
    }

    /// Creates a new `CommandStrategy` with default settings and a preset command to execute.
    pub fn command(command: ExecCommand) -> Self {
        CommandStrategy::default().with_exec_command(command)
    }

    /// Set the fail fast flag for the strategy, meaning that if the command's first run does not
    /// have the expected exit code, the strategy will exit with failure. If the flag is not set,
    /// the strategy will continue to poll the container until the expected exit code is reached.
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Set the command for executing the command on the container.
    pub fn with_exec_command(mut self, command: ExecCommand) -> Self {
        self.command = command;
        self
    }

    /// Set the poll interval for checking the container's status.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }
}

impl WaitStrategy for CommandStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()> {
        let expected_code = match self.command.clone().cmd_ready_condition {
            CmdWaitFor::Exit { code } => code,
            _ => Some(0),
        };

        loop {
            let container_state = client
                .inspect(container.id())
                .await?
                .state
                .ok_or(WaitContainerError::StateUnavailable)?;

            let is_running = container_state.running.unwrap_or_default();

            if is_running {
                let exec_result = client
                    .exec(container.id(), self.command.clone().cmd)
                    .await?;

                let inspect_result = client.inspect_exec(&exec_result.id).await?;
                let mut running = inspect_result.running.unwrap_or(false);

                loop {
                    if !running {
                        break;
                    }

                    let inspect_result = client.inspect_exec(&exec_result.id).await?;
                    let exit_code = inspect_result.exit_code;
                    running = inspect_result.running.unwrap_or(false);

                    if let Some(code) = expected_code {
                        if self.fail_fast && exit_code != expected_code {
                            return Err(WaitContainerError::UnexpectedExitCode {
                                expected: code,
                                actual: exit_code,
                            }
                            .into());
                        }
                    }

                    if exit_code == expected_code {
                        return Ok(());
                    }

                    tokio::time::sleep(self.poll_interval).await;
                }

                continue;
            }
        }
    }
}

impl Default for CommandStrategy {
    fn default() -> Self {
        Self::new()
    }
}
