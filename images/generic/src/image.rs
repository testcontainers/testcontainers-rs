use tc_core::{Container, Docker, Image, WaitError, WaitForMessage};

#[derive(Debug, PartialEq, Clone)]
pub enum WaitFor {
    Nothing,
    LogMessage(String),
}

impl WaitFor {
    fn wait<D: Docker, I: Image>(&self, container: &Container<D, I>) -> Result<(), WaitError> {
        match self {
            WaitFor::Nothing => Ok(()),
            WaitFor::LogMessage(message) => container.logs().stderr.wait_for_message(message),
        }
    }
}

#[derive(Debug)]
pub struct GenericImage {
    descriptor: String,
    arguments: Vec<String>,
    wait_for: WaitFor,
}

impl Default for GenericImage {
    fn default() -> Self {
        Self {
            descriptor: "".to_owned(),
            arguments: vec![],
            wait_for: WaitFor::Nothing,
        }
    }
}

impl GenericImage {
    pub fn new<S: Into<String>>(descriptor: S) -> GenericImage {
        Self {
            descriptor: descriptor.into(),
            arguments: vec![],
            wait_for: WaitFor::Nothing,
        }
    }

    pub fn with_wait_for(mut self, wait_for: WaitFor) -> Self {
        self.wait_for = wait_for;
        self
    }
}

impl Image for GenericImage {
    type Args = Vec<String>;

    fn descriptor(&self) -> String {
        self.descriptor.to_owned()
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        self.wait_for.wait(container).unwrap();
    }

    fn args(&self) -> Self::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self { arguments, ..self }
    }
}
