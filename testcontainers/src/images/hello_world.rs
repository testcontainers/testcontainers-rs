use crate::core::WaitFor;
use crate::Image;

#[derive(Debug, Default)]
pub struct HelloWorld;

impl Image for HelloWorld {
    type Args = ();

    fn name(&self) -> String {
        "hello-world".to_owned()
    }

    fn tag(&self) -> String {
        "latest".to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Hello from Docker!")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients;

    #[test]
    #[ignore]
    fn podman_can_run_hello_world() {
        let podman = clients::Cli::podman();

        let _container = podman.run(HelloWorld);
    }
}
