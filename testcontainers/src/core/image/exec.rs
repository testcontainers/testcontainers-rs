use crate::core::{CmdWaitFor, WaitFor};

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
