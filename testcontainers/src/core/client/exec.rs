use crate::core::logs::WaitingStreamWrapper;

pub(crate) struct ExecResult {
    pub(crate) id: String,
    pub(crate) stdout: WaitingStreamWrapper,
    pub(crate) stderr: WaitingStreamWrapper,
}

impl ExecResult {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn stdout(&mut self) -> &mut WaitingStreamWrapper {
        &mut self.stdout
    }

    pub(crate) fn stderr(&mut self) -> &mut WaitingStreamWrapper {
        &mut self.stderr
    }
}
