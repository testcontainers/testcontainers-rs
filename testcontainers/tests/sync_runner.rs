#![cfg(feature = "blocking")]

use std::time::Instant;

use testcontainers::{
    core::{
        logs::{consumer::logging_consumer::LoggingConsumer, LogFrame},
        wait::LogWaitStrategy,
        CmdWaitFor, ExecCommand, Host, IntoContainerPort, WaitFor,
    },
    runners::{SyncBuilder, SyncRunner},
    GenericBuildableImage, *,
};

fn get_server_container(msg: Option<WaitFor>) -> GenericImage {
    let generic_image = GenericBuildableImage::new("simple_web_server", "latest")
        // "Dockerfile" is included already, so adding the build context directory is all what is needed
        .with_file(
            std::fs::canonicalize("../testimages/simple_web_server").unwrap(),
            ".",
        )
        .build_image()
        .unwrap();

    let msg = msg.unwrap_or(WaitFor::message_on_stdout("server is ready"));
    generic_image.with_wait_for(msg)
}

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
        vec![WaitFor::message_on_stdout("Hello from Docker!")]
    }
}

#[test]
fn sync_can_run_hello_world() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();
    let _container = HelloWorld.start()?;
    Ok(())
}

#[cfg(feature = "http_wait")]
#[test]
fn sync_wait_for_http() -> anyhow::Result<()> {
    use crate::core::wait::HttpWaitStrategy;

    let _ = pretty_env_logger::try_init();
    use reqwest::StatusCode;

    let waitfor_http_status =
        WaitFor::http(HttpWaitStrategy::new("/").with_expected_status_code(StatusCode::OK));

    let image = get_server_container(Some(waitfor_http_status)).with_exposed_port(80.tcp());
    let _container = image.start()?;
    Ok(())
}

#[test]
fn generic_image_with_custom_entrypoint() -> anyhow::Result<()> {
    let generic = get_server_container(None);

    let node = generic.start()?;
    let port = node.get_host_port_ipv4(80.tcp())?;
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://{}:{port}", node.get_host()?))?.text()?
    );

    let generic = get_server_container(None).with_entrypoint("./bar");

    let node = generic.start()?;
    let port = node.get_host_port_ipv4(80.tcp())?;
    assert_eq!(
        "bar",
        reqwest::blocking::get(format!("http://{}:{port}", node.get_host()?))?.text()?
    );
    Ok(())
}

#[test]
fn generic_image_exposed_ports() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let target_port = 8080;

    let generic_image = GenericBuildableImage::new("no_expose_port", "latest")
        // "Dockerfile" is included already, so adding the build context directory is all what is needed
        .with_file(std::fs::canonicalize("../testimages/no_expose_port")?, ".")
        .build_image()?;

    // This server does not EXPOSE ports in its image.
    let generic_server = generic_image
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"))
        // Explicitly expose the port, which otherwise would not be available.
        .with_exposed_port(target_port.tcp());

    let node = generic_server.start()?;
    let port = node.get_host_port_ipv4(target_port.tcp())?;
    assert!(reqwest::blocking::get(format!("http://127.0.0.1:{port}"))?
        .status()
        .is_success());
    Ok(())
}

#[test]
fn generic_image_running_with_extra_hosts_added() -> anyhow::Result<()> {
    let server_1 = get_server_container(None);
    let node = server_1.start()?;
    let port = node.get_host_port_ipv4(80.tcp())?;

    let msg = WaitFor::message_on_stdout("foo");
    let server_2 = GenericImage::new("curlimages/curl", "latest")
        .with_wait_for(msg)
        .with_entrypoint("curl");

    // Override hosts for server_2 adding
    // custom-host as an alias for localhost
    let server_2 = server_2
        .with_cmd([format!("http://custom-host:{port}")])
        .with_host("custom-host", Host::HostGateway);

    server_2.start()?;
    Ok(())
}

#[test]
fn generic_image_port_not_exposed() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let target_port = 8080;

    // This image binds to 0.0.0.0:8080, does not EXPOSE ports in its dockerfile.
    let node = GenericBuildableImage::new("no_expose_port", "latest")
        // "Dockerfile" is included already, so adding the build context directory is all what is needed
        .with_file(std::fs::canonicalize("../testimages/no_expose_port")?, ".")
        .build_image()?
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"))
        .start()?;

    // Without exposing the port with `with_exposed_port()`, we cannot get a mapping to it.
    let res = node.get_host_port_ipv4(target_port.tcp());
    assert!(res.is_err());
    Ok(())
}

#[test]
fn start_multiple_containers() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("hello-world", "latest").with_wait_for(WaitFor::seconds(2));

    let _container_1 = image.clone().start()?;
    let _container_2 = image.clone().start()?;
    let _container_3 = image.start()?;
    Ok(())
}

#[test]
fn sync_run_exec() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let waitfor = WaitFor::log(LogWaitStrategy::stdout("server is ready").with_times(2));

    let image = get_server_container(Some(waitfor)).with_wait_for(WaitFor::seconds(1));
    let container = image.start()?;

    // exit regardless of the code
    let before = Instant::now();
    let res = container
        .exec(ExecCommand::new(["sleep", "2"]).with_cmd_ready_condition(CmdWaitFor::exit()))?;
    assert_eq!(res.exit_code()?, Some(0));
    assert!(
        before.elapsed().as_secs() > 1,
        "should have waited for 2 seconds"
    );

    // exit code, it waits for result
    let before = Instant::now();
    let res = container.exec(
        ExecCommand::new(["sleep", "2"]).with_cmd_ready_condition(CmdWaitFor::exit_code(0)),
    )?;
    assert_eq!(res.exit_code()?, Some(0));
    assert!(
        before.elapsed().as_secs() > 1,
        "should have waited for 2 seconds"
    );

    // stdout
    let mut res = container.exec(
        ExecCommand::new(vec!["ls".to_string()])
            .with_cmd_ready_condition(CmdWaitFor::message_on_stdout("foo")),
    )?;
    let stdout = String::from_utf8(res.stdout_to_vec()?)?;
    assert!(stdout.contains("foo"), "stdout must contain 'foo'");

    // stdout and stderr to vec
    let mut res = container.exec(ExecCommand::new([
        "/bin/bash",
        "-c",
        "echo 'stdout 1' >&1 && echo 'stderr 1' >&2 \
            && echo 'stderr 2' >&2 && echo 'stdout 2' >&1",
    ]))?;

    let stdout = String::from_utf8(res.stdout_to_vec()?)?;
    assert_eq!(stdout, "stdout 1\nstdout 2\n");

    let stderr = String::from_utf8(res.stderr_to_vec()?)?;
    assert_eq!(stderr, "stderr 1\nstderr 2\n");

    // stdout and stderr readers
    let mut res = container.exec(ExecCommand::new([
        "/bin/bash",
        "-c",
        "echo 'stdout 1' >&1 && echo 'stderr 1' >&2 \
            && echo 'stderr 2' >&2 && echo 'stdout 2' >&1",
    ]))?;

    let mut stdout = String::new();
    res.stdout().read_to_string(&mut stdout)?;
    assert_eq!(stdout, "stdout 1\nstdout 2\n");

    let mut stderr = String::new();
    res.stderr().read_to_string(&mut stderr)?;
    assert_eq!(stderr, "stderr 1\nstderr 2\n");
    Ok(())
}

#[test]
fn sync_run_with_log_consumer() -> anyhow::Result<()> {
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
        .start()?;
    rx.recv()?; // notification from consumer
    Ok(())
}

#[test]
fn sync_copy_bytes_to_container() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let container = GenericImage::new("alpine", "latest")
        .with_wait_for(WaitFor::seconds(2))
        .with_copy_to("/tmp/somefile", "foobar".to_string().into_bytes())
        .with_cmd(vec!["cat", "/tmp/somefile"])
        .start()?;

    let mut out = String::new();
    container.stdout(false).read_to_string(&mut out)?;

    assert!(out.contains("foobar"));

    Ok(())
}

#[test]
fn sync_copy_files_to_container() -> anyhow::Result<()> {
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
        .start()?;

    let mut out = String::new();
    container.stdout(false).read_to_string(&mut out)?;

    println!("{}", out);
    assert!(out.contains("foofoofoo"));
    assert!(out.contains("barbarbar"));

    Ok(())
}

#[test]
fn sync_container_is_running() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    // Container that should run until manually quit
    let container = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .start()?;

    assert!(container.is_running()?);

    container.stop()?;

    assert!(!container.is_running()?);
    Ok(())
}

#[test]
fn sync_container_exit_code() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    // Container that should run until manually quit
    let container = get_server_container(None).start()?;

    assert_eq!(container.exit_code()?, None);

    container.stop()?;

    assert_eq!(container.exit_code()?, Some(0));
    Ok(())
}
