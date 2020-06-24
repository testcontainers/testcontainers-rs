use std::io::{self, BufRead, BufReader, Read};

/// Defines error cases when waiting for a message in a stream.
#[derive(Debug)]
pub enum WaitError {
    /// Indicates the stream ended before finding the log line you were looking for.
    /// Contains all the lines that were read for debugging purposes.
    EndOfStream(Vec<String>),
    IO(io::Error),
}

impl From<io::Error> for WaitError {
    fn from(e: io::Error) -> Self {
        WaitError::IO(e)
    }
}

/// Extension trait for io::Read to wait for a message to appear in the given stream.
pub trait WaitForMessage {
    fn wait_for_message(self, message: &str) -> Result<(), WaitError>;
}

impl<T> WaitForMessage for T
where
    T: Read,
{
    fn wait_for_message(self, message: &str) -> Result<(), WaitError> {
        let logs = BufReader::new(self);
        let mut lines = vec![];

        for line in logs.lines() {
            let line = line?;

            if line.contains(message) {
                log::info!("Found message after comparing {} lines", lines.len());

                return Ok(());
            }

            lines.push(line);
        }

        log::error!(
            "Failed to find message in stream after comparing {} lines.",
            lines.len()
        );

        Err(WaitError::EndOfStream(lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_logs_when_line_contains_message_should_find_it() {
        let logs = r"
            Message one
            Message two
            Message three
        "
        .as_bytes();

        let result = logs.wait_for_message("Message three");

        assert!(result.is_ok())
    }
}
