use std::time::Duration;
use testcontainers::{
    core::WaitFor,
    images::{generic::GenericImage, hello_world::HelloWorld},
    *,
};

#[tokio::test(flavor = "multi_thread")]
async fn shiplift_can_run_hello_world() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Http::default();

    let _container = docker.run(HelloWorld).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn start_containers_in_parallel() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Http::default();

    let image = GenericImage::new("hello-world").with_wait_for(WaitFor::seconds(2));

    let run_1 = docker.run(image.clone());
    let run_2 = docker.run(image.clone());
    let run_3 = docker.run(image.clone());
    let run_4 = docker.run(image);

    let run_all = futures::future::join_all(vec![run_1, run_2, run_3, run_4]);

    // if we truly run all containers in parallel, we should finish < 5 sec
    // actually, we should be finishing in 2 seconds but that is too unstable
    // a sequential start would mean 8 seconds, hence 5 seconds proves some form of parallelism
    let timeout = Duration::from_secs(5);
    let _containers = tokio::time::timeout(timeout, run_all).await.unwrap();
}
