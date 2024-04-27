use std::{env::var, time::Duration};

/// Represents a condition that needs to be met before a container is considered ready.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum WaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a message on the stdout stream of the container's logs.
    StdOutMessage { message: String },
    /// Wait for a message on the stderr stream of the container's logs.
    StdErrMessage { message: String },
    /// Wait for a certain amount of time.
    Duration { length: Duration },
    /// Wait for the container's status to become `healthy`.
    Healthcheck,
}

impl WaitFor {
    pub fn message_on_stdout<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::StdOutMessage {
            message: message.into(),
        }
    }

    pub fn message_on_stderr<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::StdErrMessage {
            message: message.into(),
        }
    }

    pub fn seconds(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_secs(length),
        }
    }

    pub fn millis(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_millis(length),
        }
    }

    pub fn millis_in_env_var(name: &'static str) -> WaitFor {
        let additional_sleep_period = var(name).map(|value| value.parse());

        (|| {
            let length = additional_sleep_period.ok()?.ok()?;

            Some(WaitFor::Duration {
                length: Duration::from_millis(length),
            })
        })()
        .unwrap_or(WaitFor::Nothing)
    }
}
