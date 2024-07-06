#![cfg(feature = "blocking")]

use reqwest::StatusCode;
use testcontainers::{
    core::{
        logs::{consumer::logging_consumer::LoggingConsumer, LogFrame},
        wait::{HttpWaitStrategy, LogWaitStrategy},
        CmdWaitFor, ExecCommand, Host, IntoContainerPort, WaitFor,
    },
    runners::SyncRunner,
    *,
};

fn get_server_container(msg: Option<WaitFor>) -> GenericImage {
    let msg = msg.unwrap_or(WaitFor::message_on_stdout("server is ready"));
    GenericImage::new("simple_web_server", "latest").with_wait_for(msg)
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

#[test]
fn sync_wait_for_http() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericImage::new("simple_web_server", "latest")
        .with_exposed_port(80.tcp())
        .with_wait_for(WaitFor::http(
            HttpWaitStrategy::new("/").with_expected_status_code(StatusCode::OK),
        ));
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

    // This server does not EXPOSE ports in its image.
    let generic_server = GenericImage::new("no_expose_port", "latest")
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
    let generic_server = GenericImage::new("no_expose_port", "latest")
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"));
    let node = generic_server.start()?;

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

    let image = GenericImage::new("simple_web_server", "latest")
        .with_wait_for(WaitFor::log(
            LogWaitStrategy::stdout("server is ready").with_times(2),
        ))
        .with_wait_for(WaitFor::seconds(1));
    let container = image.start()?;

    // exit code, it waits for result
    let res = container.exec(
        ExecCommand::new(vec!["sleep".to_string(), "3".to_string()])
            .with_cmd_ready_condition(CmdWaitFor::exit_code(0)),
    )?;
    assert_eq!(res.exit_code()?, Some(0));

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
