use std::time::{Duration, Instant};

use bollard::Docker;
use testcontainers::{
    core::{
        logs::{consumer::logging_consumer::LoggingConsumer, LogFrame},
        wait::{CommandStrategy, ExitWaitStrategy, LogWaitStrategy},
        CmdWaitFor, ContainerState, ExecCommand, WaitFor,
    },
    runners::AsyncRunner,
    GenericImage, Image, ImageExt,
};
use tokio::io::AsyncReadExt;

#[derive(Debug, Default)]
pub struct HelloWorld;

impl Image for HelloWorld {
    fn name(&self) -> &str {
        "hello-world"
    }

    fn tag(&self) -> &str {
        "latest"
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout("Hello from Docker!"),
            WaitFor::exit(ExitWaitStrategy::new().with_exit_code(0)),
        ]
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn bollard_can_run_hello_world_with_multi_thread() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let _container = HelloWorld.start().await?;
    Ok(())
}

async fn cleanup_hello_world_image() -> anyhow::Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    futures::future::join_all(
        docker
            .list_images::<String>(None)
            .await?
            .into_iter()
            .flat_map(|image| image.repo_tags.into_iter())
            .filter(|tag| tag.starts_with("hello-world"))
            .map(|tag| async {
                let tag_captured = tag;
                docker.remove_image(&tag_captured, None, None).await
            }),
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn bollard_pull_missing_image_hello_world() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();
    cleanup_hello_world_image().await?;
    let _container = HelloWorld.start().await?;
    Ok(())
}

#[tokio::test]
async fn explicit_call_to_pull_missing_image_hello_world() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();
    cleanup_hello_world_image().await?;
    let _container = HelloWorld.pull_image().await?.start().await?;
    Ok(())
}

#[tokio::test]
async fn start_containers_in_parallel() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("hello-world", "latest").with_wait_for(WaitFor::seconds(2));

    // Make sure the image is already pulled, since otherwise pulling it may cause the deadline
    // below to be exceeded.
    let _ = image.clone().pull_image().await?;

    let run_1 = image.clone().start();
    let run_2 = image.clone().start();
    let run_3 = image.clone().start();
    let run_4 = image.start();

    let run_all = futures::future::join_all(vec![run_1, run_2, run_3, run_4]);

    // if we truly run all containers in parallel, we should finish < 5 sec
    // actually, we should be finishing in 2 seconds but that is too unstable
    // a sequential start would mean 8 seconds, hence 5 seconds proves some form of parallelism
    let timeout = Duration::from_secs(5);
    let _containers = tokio::time::timeout(timeout, run_all).await?;
    Ok(())
}

#[tokio::test]
async fn async_wait_for_successful_command_strategy() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("postgres", "latest").with_wait_for(WaitFor::command(
        ExecCommand::new(["pg_isready"]).with_cmd_ready_condition(CmdWaitFor::exit_code(0)),
    ));
    let container = image
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "db")
        .start()
        .await?;

    let mut out = String::new();
    container.stdout(false).read_to_string(&mut out).await?;

    assert!(
        out.contains("server started"),
        "stdout must contain 'server started'"
    );

    // if the container.exec exits with 0, then it means the wait_for successful command strategy
    // worked
    //assert_eq!(res.exit_code().await?, Some(0));

    Ok(())
}

#[tokio::test]
async fn async_run_exec() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stderr("server will be listening to"))
        .with_wait_for(WaitFor::log(
            LogWaitStrategy::stdout("server is ready").with_times(2),
        ))
        .with_wait_for(WaitFor::seconds(1));
    let container = image.start().await?;

    // exit regardless of the code
    let before = Instant::now();
    let res = container
        .exec(ExecCommand::new(["sleep", "2"]).with_cmd_ready_condition(CmdWaitFor::exit()))
        .await?;
    assert_eq!(res.exit_code().await?, Some(0));
    assert!(
        before.elapsed().as_secs() > 1,
        "should have waited for 2 seconds"
    );

    // exit code, it waits for result
    let before = Instant::now();
    let res = container
        .exec(ExecCommand::new(["sleep", "2"]).with_cmd_ready_condition(CmdWaitFor::exit_code(0)))
        .await?;
    assert_eq!(res.exit_code().await?, Some(0));
    assert!(
        before.elapsed().as_secs() > 1,
        "should have waited for 2 seconds"
    );

    // stdout
    let mut res = container
        .exec(
            ExecCommand::new(["ls"]).with_cmd_ready_condition(CmdWaitFor::message_on_stdout("foo")),
        )
        .await?;

    let stdout = String::from_utf8(res.stdout_to_vec().await?)?;
    assert!(stdout.contains("foo"), "stdout must contain 'foo'");

    // stdout and stderr readers
    let mut res = container
        .exec(ExecCommand::new([
            "/bin/bash",
            "-c",
            "echo 'stdout 1' >&1 && echo 'stderr 1' >&2 \
            && echo 'stderr 2' >&2 && echo 'stdout 2' >&1",
        ]))
        .await?;

    let mut stdout = String::new();
    res.stdout().read_to_string(&mut stdout).await?;
    assert_eq!(stdout, "stdout 1\nstdout 2\n");

    let mut stderr = String::new();
    res.stderr().read_to_string(&mut stderr).await?;
    assert_eq!(stderr, "stderr 1\nstderr 2\n");
    Ok(())
}

#[cfg(feature = "http_wait")]
#[tokio::test]
async fn async_wait_for_http() -> anyhow::Result<()> {
    use reqwest::StatusCode;
    use testcontainers::core::{wait::HttpWaitStrategy, IntoContainerPort};

    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_exposed_port(80.tcp())
        .with_wait_for(WaitFor::http(
            HttpWaitStrategy::new("/").with_expected_status_code(StatusCode::OK),
        ));
    let _container = image.start().await?;
    Ok(())
}

#[tokio::test]
async fn async_run_exec_fails_due_to_unexpected_code() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .with_wait_for(WaitFor::seconds(1));
    let container = image.start().await?;

    // exit code, it waits for result
    let res = container
        .exec(
            ExecCommand::new(vec!["ls".to_string()])
                .with_cmd_ready_condition(CmdWaitFor::exit_code(-1)),
        )
        .await;
    assert!(res.is_err());
    Ok(())
}

#[tokio::test]
async fn async_run_with_log_consumer() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let _container = HelloWorld
        .with_log_consumer(move |frame: &LogFrame| {
            // notify when the expected message is found
            if String::from_utf8_lossy(frame.bytes()) == "Hello from Docker!\n" {
                let _ = tx.send(());
            }
        })
        .with_log_consumer(LoggingConsumer::new().with_stderr_level(log::Level::Error))
        .start()
        .await?;
    rx.recv()?; // notification from consumer
    Ok(())
}

#[tokio::test]
async fn async_copy_bytes_to_container() -> anyhow::Result<()> {
    let container = GenericImage::new("alpine", "latest")
        .with_wait_for(WaitFor::seconds(2))
        .with_copy_to("/tmp/somefile", "foobar".to_string().into_bytes())
        .with_cmd(vec!["cat", "/tmp/somefile"])
        .start()
        .await?;

    let mut out = String::new();
    container.stdout(false).read_to_string(&mut out).await?;

    assert!(out.contains("foobar"));

    Ok(())
}

#[tokio::test]
async fn async_copy_files_to_container() -> anyhow::Result<()> {
    let temp_dir = temp_dir::TempDir::new()?;
    let f1 = temp_dir.child("foo.txt");

    let sub_dir = temp_dir.child("subdir");
    std::fs::create_dir(&sub_dir)?;
    let mut f2 = sub_dir.clone();
    f2.push("bar.txt");

    std::fs::write(&f1, "foofoofoo")?;
    std::fs::write(&f2, "barbarbar")?;

    let container = GenericImage::new("alpine", "latest")
        .with_wait_for(WaitFor::seconds(2))
        .with_copy_to("/tmp/somefile", f1)
        .with_copy_to("/", temp_dir.path())
        .with_cmd(vec!["cat", "/tmp/somefile", "&&", "cat", "/subdir/bar.txt"])
        .start()
        .await?;

    let mut out = String::new();
    container.stdout(false).read_to_string(&mut out).await?;

    println!("{}", out);
    assert!(out.contains("foofoofoo"));
    assert!(out.contains("barbarbar"));

    Ok(())
}

#[tokio::test]
async fn async_container_is_running() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    // Container that should run until manually quit
    let container = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .start()
        .await?;

    assert!(container.is_running().await?);

    container.stop().await?;

    assert!(!container.is_running().await?);
    Ok(())
}

#[tokio::test]
async fn async_container_exit_code() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    // Container that should run until manually quit
    let container = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .start()
        .await?;

    assert_eq!(container.exit_code().await?, None);

    container.stop().await?;

    assert_eq!(container.exit_code().await?, Some(0));
    Ok(())
}
