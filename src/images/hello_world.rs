use crate::{core::WaitFor, Image};

#[derive(Debug, Default)]
pub struct HelloWorld;

impl Image for HelloWorld {
    type Args = Vec<String>;

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
