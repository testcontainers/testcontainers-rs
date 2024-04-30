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

#[derive(Debug, Default)]
pub struct ExecWithCommand {
    pub(crate) cmd: Vec<String>,
    pub(crate) attach_stdout: Option<bool>,
    pub(crate) attach_stderr: Option<bool>,
    pub(crate) container_ready_conditions: Vec<WaitFor>,
}
impl ExecWithCommand {
    /// Command to be executed
    pub fn new(cmd: Vec<String>) -> Self {
        Self {
            cmd,
            attach_stdout: None,
            attach_stderr: None,
            container_ready_conditions: vec![],
        }
    }

    /// Whether to attach stdout of a running container
    pub fn with_attach_stdout(mut self, attach_stdout: Option<bool>) -> Self {
        self.attach_stdout = attach_stdout;
        self
    }

    /// Whether to attach stderr of a running container
    pub fn with_attach_stderr(mut self, attach_stderr: Option<bool>) -> Self {
        self.attach_stderr = attach_stderr;
        self
    }

    /// Conditions to be checked on related container
    pub fn with_container_ready_conditions(mut self, ready_conditions: Vec<WaitFor>) -> Self {
        self.container_ready_conditions = ready_conditions;
        self
    }
}
