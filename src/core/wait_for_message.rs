use std::io::{self, BufRead, BufReader, Read};

/// Defines error cases when waiting for a message in a stream.
#[derive(Debug)]
pub enum WaitError {
    EndOfStream,
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

        let mut iter = logs.lines().into_iter();

        let mut number_of_compared_lines = 0;

        while let Some(line) = iter.next() {
            let line = line?;
            number_of_compared_lines += 1;

            if line.contains(message) {
                log::info!(
                    "Found message after comparing {} lines",
                    number_of_compared_lines
                );

                return Ok(());
            }
        }

        log::error!(
            "Failed to find message in stream after comparing {} lines.",
            number_of_compared_lines
        );

        Err(WaitError::EndOfStream)
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
