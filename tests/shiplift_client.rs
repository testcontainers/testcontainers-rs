use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use spectral::prelude::*;

use async_trait::async_trait;
use testcontainers::core::Port;
use testcontainers::core::WaitForMessageAsync;
use testcontainers::*;

#[derive(Default)]
struct HelloWorld;

#[async_trait]
impl ImageAsync for HelloWorld {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        String::from("hello-world")
    }

    async fn wait_until_ready<D: DockerAsync + Sync>(
        &self,
        container: &ContainerAsync<'_, D, Self>,
    ) {
        let mut logstream_stdout = container.logs().await.stdout;
        logstream_stdout
            .wait_for_message_async("Hello from Docker!")
            .await
            .unwrap();
    }

    fn args(&self) -> <Self as ImageAsync>::Args {
        vec![]
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn ports(&self) -> Option<Vec<Port>> {
        None
    }

    fn with_args(self, _arguments: <Self as ImageAsync>::Args) -> Self {
        self
    }
}

#[test]
fn should_wait_for_at_least_one_second_before_fetching_logs_shiplift() {
    tokio_test::block_on(async {
        let _ = pretty_env_logger::try_init();

        let docker = clients::Shiplift::new();

        let before_run = Instant::now();

        let container = docker.run(HelloWorld).await;

        let after_run = Instant::now();

        let before_logs = Instant::now();

        // this probably doesn't work anymore in async
        docker.logs(container.id()).await;

        let after_logs = Instant::now();

        assert_that(&(after_run - before_run)).is_greater_than(Duration::from_secs(1));
        assert_that(&(after_logs - before_logs)).is_less_than(Duration::from_secs(1));
    })
}
