use crate::core::logs::LogStreamAsync;

pub(crate) struct ExecResult<'a> {
    pub(crate) id: String,
    pub(crate) stdout: LogStreamAsync<'a>,
    pub(crate) stderr: LogStreamAsync<'a>,
}

impl<'a> ExecResult<'a> {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn stdout(&mut self) -> &mut LogStreamAsync<'a> {
        &mut self.stdout
    }

    pub(crate) fn stderr(&mut self) -> &mut LogStreamAsync<'a> {
        &mut self.stderr
    }
}
