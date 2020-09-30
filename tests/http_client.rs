use testcontainers::{images::hello_world::HelloWorld, *};

#[tokio::test(flavor = "multi_thread")]
async fn shiplift_can_run_hello_world() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Http::default();

    let _container = docker.run(HelloWorld).await;
}
