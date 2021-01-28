use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use spectral::prelude::*;

use testcontainers::*;

#[derive(Default)]
struct HelloWorld;

impl Image for HelloWorld {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        String::from("hello-world")
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Hello from Docker!")
            .unwrap();
    }

    fn args(&self) -> <Self as Image>::Args {
        vec![]
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn with_args(self, _arguments: <Self as Image>::Args) -> Self {
        self
    }
}

#[test]
fn should_wait_for_at_least_one_second_before_fetching_logs() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Cli::default();

    let before_run = Instant::now();

    let container = docker.run(HelloWorld);

    let after_run = Instant::now();

    let before_logs = Instant::now();

    docker.logs(container.id());

    let after_logs = Instant::now();

    assert_that(&(after_run - before_run)).is_greater_than(Duration::from_secs(1));
    assert_that(&(after_logs - before_logs)).is_less_than(Duration::from_secs(1));
}
