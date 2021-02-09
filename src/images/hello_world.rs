use crate::{core::WaitFor, Image};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct HelloWorld;

impl Image for HelloWorld {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        String::from("hello-world")
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Hello from Docker!")]
    }

    fn args(&self) -> Self::Args {
        vec![]
    }

    fn with_args(self, _: <Self as Image>::Args) -> Self {
        self
    }
}
