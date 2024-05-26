use crate::core::logs::LogStreamAsync;

pub(crate) struct ExecResult {
    pub(crate) id: String,
    pub(crate) stdout: LogStreamAsync,
    pub(crate) stderr: LogStreamAsync,
}

impl ExecResult {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn stdout(&mut self) -> &mut LogStreamAsync {
        &mut self.stdout
    }

    pub(crate) fn stderr(&mut self) -> &mut LogStreamAsync {
        &mut self.stderr
    }
}
