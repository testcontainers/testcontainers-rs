use std::time::Duration;

use crate::core::WaitFor;

#[derive(Debug)]
pub struct ExecCommand {
    pub(crate) cmd: Vec<String>,
    pub(crate) cmd_ready_condition: CmdWaitFor,
    pub(crate) container_ready_conditions: Vec<WaitFor>,
}

impl ExecCommand {
    /// Command to be executed
    pub fn new(cmd: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            cmd: cmd.into_iter().map(Into::into).collect(),
            cmd_ready_condition: CmdWaitFor::Nothing,
            container_ready_conditions: vec![],
        }
    }

    /// Conditions to be checked on related container
    pub fn with_container_ready_conditions(mut self, ready_conditions: Vec<WaitFor>) -> Self {
        self.container_ready_conditions = ready_conditions;
        self
    }

    /// Conditions to be checked on executed command output
    pub fn with_cmd_ready_condition(mut self, ready_conditions: impl Into<CmdWaitFor>) -> Self {
        self.cmd_ready_condition = ready_conditions.into();
        self
    }
}

impl Default for ExecCommand {
    fn default() -> Self {
        Self::new(Vec::<String>::new())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CmdWaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a message on the stdout stream of the command's output.
    StdOutMessage { message: String },
    /// Wait for a message on the stderr stream of the command's output.
    StdErrMessage { message: String },
    /// Wait for a message on the stdout or stderr stream of the command's output.
    StdOutOrErrMessage { message: String },
    /// Wait for a certain amount of time.
    Duration { length: Duration },
    /// Wait for the command's exit code to be equal to the provided one.
    ExitCode { code: i64 },
}

impl CmdWaitFor {
    pub fn message_on_stdout<S: Into<String>>(message: S) -> Self {
        Self::StdOutMessage {
            message: message.into(),
        }
    }

    pub fn message_on_stderr<S: Into<String>>(message: S) -> Self {
        Self::StdErrMessage {
            message: message.into(),
        }
    }

    pub fn message_on_stdout_or_stderr<S: Into<String>>(message: S) -> Self {
        Self::StdOutOrErrMessage {
            message: message.into(),
        }
    }

    pub fn exit_code(code: i64) -> Self {
        Self::ExitCode { code }
    }

    pub fn seconds(length: u64) -> Self {
        Self::Duration {
            length: Duration::from_secs(length),
        }
    }

    pub fn millis(length: u64) -> Self {
        Self::Duration {
            length: Duration::from_millis(length),
        }
    }
}

impl From<WaitFor> for CmdWaitFor {
    fn from(wait_for: WaitFor) -> Self {
        match wait_for {
            WaitFor::Nothing => Self::Nothing,
            WaitFor::StdOutMessage { message } => Self::StdOutMessage { message },
            WaitFor::StdErrMessage { message } => Self::StdErrMessage { message },
            WaitFor::Duration { length } => Self::Duration { length },
            WaitFor::Healthcheck => Self::ExitCode { code: 0 },
        }
    }
}
