use std::collections::HashMap;
use tc_core::{Container, Docker, Image, WaitError, WaitForMessage};

#[derive(Debug, PartialEq, Clone)]
pub enum WaitFor {
    Nothing,
    LogMessage { message: String, stream: Stream },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stream {
    StdOut,
    StdErr,
}

impl WaitFor {
    pub fn message_on_stdout<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::LogMessage {
            message: message.into(),
            stream: Stream::StdOut,
        }
    }

    pub fn message_on_stderr<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::LogMessage {
            message: message.into(),
            stream: Stream::StdErr,
        }
    }

    fn wait<D: Docker, I: Image>(&self, container: &Container<D, I>) -> Result<(), WaitError> {
        match self {
            WaitFor::Nothing => Ok(()),
            WaitFor::LogMessage { message, stream } => match stream {
                Stream::StdOut => container.logs().stdout.wait_for_message(message),
                Stream::StdErr => container.logs().stderr.wait_for_message(message),
            },
        }
    }
}

#[derive(Debug)]
pub struct GenericImage {
    descriptor: String,
    arguments: Vec<String>,
    env_vars: HashMap<String, String>,
    wait_for: WaitFor,
}

impl Default for GenericImage {
    fn default() -> Self {
        Self {
            descriptor: "".to_owned(),
            arguments: vec![],
            env_vars: HashMap::new(),
            wait_for: WaitFor::Nothing,
        }
    }
}

impl GenericImage {
    pub fn new<S: Into<String>>(descriptor: S) -> GenericImage {
        Self {
            descriptor: descriptor.into(),
            arguments: vec![],
            env_vars: HashMap::new(),
            wait_for: WaitFor::Nothing,
        }
    }

    pub fn with_env_var<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    pub fn with_wait_for(mut self, wait_for: WaitFor) -> Self {
        self.wait_for = wait_for;
        self
    }
}

impl Image for GenericImage {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;

    fn descriptor(&self) -> String {
        self.descriptor.to_owned()
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        self.wait_for.wait(container).unwrap();
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_return_env_vars() {
        let image = GenericImage::new("hello")
            .with_env_var("one-key", "one-value")
            .with_env_var("two-key", "two-value");

        let env_vars = image.env_vars();
        assert_eq!(2, env_vars.len());
        assert_eq!("one-value", env_vars.get("one-key").unwrap());
        assert_eq!("two-value", env_vars.get("two-key").unwrap());
    }

}
