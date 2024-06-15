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
    /// Wait for the command's exit code to be equal to the provided one.
    ExitCode { code: i64 },
}

impl CmdWaitFor {
    pub fn message_on_stdout(message: impl AsRef<[u8]>) -> Self {
        Self::StdOutMessage {
            message: Bytes::from(message.as_ref().to_vec()),
        }
    }

    pub fn message_on_stderr(message: impl AsRef<[u8]>) -> Self {
        Self::StdErrMessage {
            message: Bytes::from(message.as_ref().to_vec()),
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
