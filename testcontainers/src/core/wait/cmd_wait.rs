use std::time::Duration;

use bytes::Bytes;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CmdWaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a message on the stdout stream of the command's output.
    StdOutMessage { message: Bytes },
    /// Wait for a message on the stderr stream of the command's output.
    StdErrMessage { message: Bytes },
    /// Wait for a certain amount of time.
    Duration { length: Duration },
    /// Wait for the command to exit and optionally check the exit code.
    Exit { code: Option<i64> },
}

impl CmdWaitFor {
    /// Wait for a message on the stdout stream of the command's output.
    pub fn message_on_stdout(message: impl AsRef<[u8]>) -> Self {
        Self::StdOutMessage {
            message: Bytes::from(message.as_ref().to_vec()),
        }
    }

    /// Wait for a message on the stderr stream of the command's output.
    pub fn message_on_stderr(message: impl AsRef<[u8]>) -> Self {
        Self::StdErrMessage {
            message: Bytes::from(message.as_ref().to_vec()),
        }
    }

    /// Wait for the command to exit (regardless of the exit code).
    pub fn exit() -> Self {
        Self::Exit { code: None }
    }

    /// Wait for the command's exit code to be equal to the provided one.
    pub fn exit_code(code: i64) -> Self {
        Self::Exit { code: Some(code) }
    }

    /// Wait for a certain amount of time.
    pub fn duration(duration: Duration) -> Self {
        Self::Duration { length: duration }
    }

    /// Wait for a certain amount of time (in seconds).
    pub fn seconds(secs: u64) -> Self {
        Self::duration(Duration::from_secs(secs))
    }

    /// Wait for a certain amount of time (in millis)
    pub fn millis(millis: u64) -> Self {
        Self::duration(Duration::from_millis(millis))
    }
}
