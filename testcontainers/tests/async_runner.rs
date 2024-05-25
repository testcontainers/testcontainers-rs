use std::time::Duration;

use bollard::Docker;
use testcontainers::{
    core::{CmdWaitFor, ExecCommand, WaitFor},
    runners::AsyncRunner,
    GenericImage, *,
};
use tokio::io::AsyncReadExt;

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

#[tokio::test(flavor = "multi_thread")]
async fn bollard_can_run_hello_world_with_multi_thread() {
    let _ = pretty_env_logger::try_init();

    let _container = HelloWorld.start().await;
}

async fn cleanup_hello_world_image() {
    let docker = Docker::connect_with_unix_defaults().unwrap();
    futures::future::join_all(
        docker
            .list_images::<String>(None)
            .await
            .unwrap()
            .into_iter()
            .flat_map(|image| image.repo_tags.into_iter())
            .filter(|tag| tag.starts_with("hello-world"))
            .map(|tag| async {
                let tag_captured = tag;
                docker.remove_image(&tag_captured, None, None).await
            }),
    )
    .await;
}

#[tokio::test]
async fn bollard_pull_missing_image_hello_world() {
    let _ = pretty_env_logger::try_init();
    cleanup_hello_world_image().await;
    let _container = RunnableImage::from(HelloWorld).start().await;
}

#[tokio::test]
async fn explicit_call_to_pull_missing_image_hello_world() {
    let _ = pretty_env_logger::try_init();
    cleanup_hello_world_image().await;
    let _container = RunnableImage::from(HelloWorld)
        .pull_image()
        .await
        .unwrap()
        .start()
        .await
        .unwrap();
}

#[tokio::test]
async fn start_containers_in_parallel() {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("hello-world", "latest").with_wait_for(WaitFor::seconds(2));

    let run_1 = image.clone().start();
    let run_2 = image.clone().start();
    let run_3 = image.clone().start();
    let run_4 = image.start();

    let run_all = futures::future::join_all(vec![run_1, run_2, run_3, run_4]);

    // if we truly run all containers in parallel, we should finish < 5 sec
    // actually, we should be finishing in 2 seconds but that is too unstable
    // a sequential start would mean 8 seconds, hence 5 seconds proves some form of parallelism
    let timeout = Duration::from_secs(5);
    let _containers = tokio::time::timeout(timeout, run_all).await.unwrap();
}

#[tokio::test]
async fn async_run_exec() {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .with_wait_for(WaitFor::seconds(1));
    let container = image.start().await.unwrap();

    // exit code, it waits for result
    let res = container
        .exec(ExecCommand::new(["sleep", "3"]).with_cmd_ready_condition(CmdWaitFor::exit_code(0)))
        .await
        .unwrap();
    assert_eq!(res.exit_code().await.unwrap(), Some(0));

    // stdout
    let mut res = container
        .exec(
            ExecCommand::new(["ls"]).with_cmd_ready_condition(CmdWaitFor::message_on_stdout("foo")),
        )
        .await
        .unwrap();

    let stdout = String::from_utf8(res.stdout_to_vec().await.unwrap()).unwrap();
    assert!(stdout.contains("foo"), "stdout must contain 'foo'");

    // stdout and stderr readers
    let mut res = container
        .exec(ExecCommand::new([
            "/bin/bash",
            "-c",
            "echo 'stdout 1' >&1 && echo 'stderr 1' >&2 \
            && echo 'stderr 2' >&2 && echo 'stdout 2' >&1",
        ]))
        .await
        .unwrap();

    let mut stdout = String::new();
    res.stdout().read_to_string(&mut stdout).await.unwrap();
    assert_eq!(stdout, "stdout 1\nstdout 2\n");

    let mut stderr = String::new();
    res.stderr().read_to_string(&mut stderr).await.unwrap();
    assert_eq!(stderr, "stderr 1\nstderr 2\n");
}

#[tokio::test]
async fn async_run_exec_fails_due_to_unexpected_code() {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .with_wait_for(WaitFor::seconds(1));
    let container = image.start().await.unwrap();

    // exit code, it waits for result
    let res = container
        .exec(
            ExecCommand::new(vec!["ls".to_string()])
                .with_cmd_ready_condition(CmdWaitFor::exit_code(-1)),
        )
        .await;
    assert!(res.is_err());
}
