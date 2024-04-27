use crate::core::WaitFor;

#[derive(Debug)]
pub struct ExecCommand {
    pub(crate) cmd: Vec<String>,
    pub(crate) cmd_ready_condition: WaitFor,
    pub(crate) container_ready_conditions: Vec<WaitFor>,
}

impl ExecCommand {
    /// Command to be executed
    pub fn new(cmd: Vec<String>) -> Self {
        Self {
            cmd,
            cmd_ready_condition: WaitFor::Nothing,
            container_ready_conditions: vec![],
        }
    }

    /// Conditions to be checked on related container
    pub fn with_container_ready_conditions(mut self, ready_conditions: Vec<WaitFor>) -> Self {
        self.container_ready_conditions = ready_conditions;
        self
    }

    /// Conditions to be checked on executed command output
    pub fn with_cmd_ready_condition(mut self, ready_conditions: WaitFor) -> Self {
        self.cmd_ready_condition = ready_conditions;
        self
    }
}

impl Default for ExecCommand {
    fn default() -> Self {
        Self::new(vec![])
    }
}
