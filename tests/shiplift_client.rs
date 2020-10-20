use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

use spectral::prelude::*;

use async_trait::async_trait;
use testcontainers::core::Port;
use testcontainers::core::WaitForMessageAsync;
use testcontainers::images::generic_async::GenericImageAsync;
use testcontainers::*;

#[derive(Default)]
struct HelloWorld {
    volumes: HashMap<String, String>,
    env_vars: HashMap<String, String>,
}

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
        self.volumes.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.env_vars.clone()
    }

    fn ports(&self) -> Option<Vec<Port>> {
        None
    }

    fn with_args(self, _arguments: <Self as ImageAsync>::Args) -> Self {
        self
    }
}

#[tokio::test(threaded_scheduler)]
async fn should_wait_for_at_least_one_second_before_fetching_logs_shiplift() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Shiplift::new();

    let before_run = Instant::now();

    let container = docker.run(HelloWorld::default()).await;

    let after_run = Instant::now();

    let before_logs = Instant::now();

    docker.logs(container.id()).await;

    let after_logs = Instant::now();

    assert_that(&(after_run - before_run)).is_greater_than(Duration::from_secs(1));
    assert_that(&(after_logs - before_logs)).is_less_than(Duration::from_secs(1));
}

#[tokio::test(threaded_scheduler)]
async fn shiplift_run_command_should_include_env_and_volumes() {
    let mut volumes = HashMap::new();
    volumes.insert("/tmp".to_owned(), "/hostmp".to_owned());

    let mut env_vars = HashMap::new();
    env_vars.insert("one-key".to_owned(), "one-value".to_owned());
    env_vars.insert("two-key".to_owned(), "two-value".to_owned());

    let image = HelloWorld { volumes, env_vars };
    let run_args = RunArgs::default();

    let docker = clients::Shiplift::new();
    let container = docker.run_with_args(image, run_args).await;

    // inspect volume and env
    let container_details = docker
        .client
        .containers()
        .get(container.id())
        .inspect()
        .await
        .unwrap();

    let envs = container_details.config.env.unwrap();
    assert_that!(&envs).contains(&"one-key=one-value".into());
    assert_that!(&envs).contains(&"two-key=two-value".into());
}

#[tokio::test(threaded_scheduler)]
async fn shiplift_run_command_should_expose_all_ports_if_no_explicit_mapping_requested() {
    let image = HelloWorld::default();
    let docker = clients::Shiplift::new();
    let container = docker.run(image).await;

    // inspect volume and env
    let container_details = docker
        .client
        .containers()
        .get(container.id())
        .inspect()
        .await
        .unwrap();

    assert_that!(container_details.host_config.publish_all_ports).is_equal_to(true);
}

#[tokio::test(threaded_scheduler)]
async fn shiplift_run_command_should_expose_only_requested_ports() {
    let image = GenericImageAsync::new("hello-world")
        .with_mapped_port((123, 456))
        .with_mapped_port((555, 888));

    let docker = clients::Shiplift::new();
    let container = docker.run(image).await;

    // inspect volume and env
    let container_details = docker
        .client
        .containers()
        .get(container.id())
        .inspect()
        .await
        .unwrap();

    let port_bindings = container_details.host_config.port_bindings.unwrap();
    assert_that!(&port_bindings).contains_key(&"456/tcp".into());
    assert_that!(&port_bindings).contains_key(&"888/tcp".into());
}
