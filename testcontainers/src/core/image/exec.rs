use std::collections::HashMap;

use crate::core::{CmdWaitFor, WaitFor};

#[derive(Debug)]
pub struct ExecCommand {
    pub(crate) cmd: Vec<String>,
    pub(crate) cmd_ready_condition: CmdWaitFor,
    pub(crate) container_ready_conditions: Vec<WaitFor>,
    pub(crate) env_vars: HashMap<String, String>,
}

impl ExecCommand {
    /// Command to be executed
    pub fn new(cmd: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            cmd: cmd.into_iter().map(Into::into).collect(),
            cmd_ready_condition: CmdWaitFor::Nothing,
            container_ready_conditions: vec![],
            env_vars: HashMap::new(),
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

    /// Sets environment variables for the exec command.
    ///
    /// These are passed directly to the Docker exec API as the `Env` field,
    /// making them available in the executed process's environment.
    pub fn with_env_vars(
        mut self,
        env_vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.env_vars
            .extend(env_vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }
}

impl Default for ExecCommand {
    fn default() -> Self {
        Self::new(Vec::<String>::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_env_vars() {
        let exec = ExecCommand::new(["echo", "hello"])
            .with_env_vars([("KEY1", "value1"), ("KEY2", "value2")]);

        assert_eq!(exec.env_vars.get("KEY1").unwrap(), "value1");
        assert_eq!(exec.env_vars.get("KEY2").unwrap(), "value2");
        assert_eq!(exec.cmd, vec!["echo", "hello"]);
    }
}
