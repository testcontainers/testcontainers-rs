use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    thread,
};

use anyhow::Result;
use testcontainers::{
    core::{ExecCommand, Host},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt, TestcontainersError,
};
use ulid::Ulid;

const ALPINE_IMAGE: &str = "alpine";
const ALPINE_TAG: &str = "3.19";
const HOST_ALIAS: &str = "host.testcontainers.internal";

fn host_url(port: u16) -> String {
    format!("http://{HOST_ALIAS}:{port}")
}

fn base_alpine_image() -> GenericImage {
    GenericImage::new(ALPINE_IMAGE, ALPINE_TAG)
}

fn wget_host_with_timeout(port: u16) -> ExecCommand {
    let mut args = vec![
        "wget".to_string(),
        "-qO-".to_string(),
        "-T".to_string(),
        "2".to_string(),
        "-t".to_string(),
        "1".to_string(),
    ];
    args.push(host_url(port));
    ExecCommand::new(args)
}

fn ping_once(target: &str) -> ExecCommand {
    ExecCommand::new(["ping", "-c", "1", target])
}

fn respond_once(mut stream: TcpStream, body: &'static str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Starts a test HTTP server that responds with the given body and returns the port number.
fn start_test_http_server(response_body: &'static str) -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();

    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            respond_once(stream, response_body);
        }
    });

    Ok(port)
}

/// Creates a test container with the specified name, network, and exposed host port.
async fn create_test_container(
    name: &str,
    network: &str,
    exposed_port: u16,
) -> Result<ContainerAsync<GenericImage>> {
    let image = base_alpine_image()
        .with_entrypoint("/bin/sh")
        .with_cmd(["-c", "sleep 60"])
        .with_container_name(name)
        .with_network(network)
        .with_exposed_host_port(exposed_port);

    image.start().await.map_err(|e| e.into())
}

/// Asserts that a container can access the given host port and receives the expected response.
async fn assert_can_access(
    container: &ContainerAsync<GenericImage>,
    port: u16,
    expected_response: &str,
) -> Result<()> {
    let mut exec = container.exec(wget_host_with_timeout(port)).await?;
    let body = String::from_utf8(exec.stdout_to_vec().await?)?;
    assert_eq!(body, expected_response);
    assert_eq!(exec.exit_code().await?, Some(0));
    Ok(())
}

/// Asserts that a container cannot access the given host port.
async fn assert_cannot_access(container: &ContainerAsync<GenericImage>, port: u16) -> Result<()> {
    let exit = container
        .exec(wget_host_with_timeout(port))
        .await?
        .exit_code()
        .await?;
    assert_ne!(exit, Some(0));
    Ok(())
}

/// Verifies a single container can reach a host service exposed through one requested port.
#[tokio::test]
async fn exposes_single_host_port() -> Result<()> {
    let _ = pretty_env_logger::try_init();

    let port = start_test_http_server("hello-from-host")?;

    let image = base_alpine_image()
        .with_entrypoint("/bin/sh")
        .with_cmd(["-c", "sleep 30"])
        .with_exposed_host_port(port);

    let container = image.start().await?;

    assert_can_access(&container, port, "hello-from-host").await?;

    Ok(())
}

/// Ensures multiple host ports requested by the same container tunnel traffic to the correct services.
#[tokio::test]
async fn exposes_multiple_host_ports() -> Result<()> {
    let _ = pretty_env_logger::try_init();

    let port_a = start_test_http_server("alpha")?;
    let port_b = start_test_http_server("bravo")?;

    let image = base_alpine_image()
        .with_entrypoint("/bin/sh")
        .with_cmd(["-c", "sleep 30"])
        .with_exposed_host_ports([port_a, port_b]);

    let container = image.start().await?;

    assert_can_access(&container, port_a, "alpha").await?;
    assert_can_access(&container, port_b, "bravo").await?;

    Ok(())
}

/// Confirms configuring `host.testcontainers.internal` manually prevents host port exposure setup.
#[tokio::test]
async fn fails_when_alias_conflicts() {
    let image = base_alpine_image()
        .with_host(HOST_ALIAS, Host::HostGateway)
        .with_exposed_host_port(8080);

    let start_err = image.start().await.unwrap_err();
    match start_err {
        TestcontainersError::Other(message) => {
            let msg = message.to_string();
            assert!(
                msg.contains("host port exposure"),
                "unexpected error message: {msg}"
            );
        }
        other => panic!("unexpected error variant: {:?}", other),
    }
}

/// Validates the runner rejects attempts to expose the reserved SSH port used by the sidecar.
#[tokio::test]
async fn fails_when_exposing_reserved_port() {
    let image = base_alpine_image().with_exposed_host_port(22);

    let start_err = image.start().await.unwrap_err();
    match start_err {
        TestcontainersError::Other(message) => {
            let msg = message.to_string();
            assert!(
                msg.contains("SSH port is reserved"),
                "unexpected error message: {msg}"
            );
        }
        other => panic!("unexpected error variant: {:?}", other),
    }
}

/// Validates that host port exposure is scoped per container and doesn't leak between containers.
#[tokio::test]
async fn host_port_exposure_is_scoped_per_container() -> Result<()> {
    let _ = pretty_env_logger::try_init();

    // Create two distinct host services on different ports
    let first_host_port = start_test_http_server("first-host-service")?;
    let second_host_port = start_test_http_server("second-host-service")?;

    // Generate unique identifiers for network and containers to avoid conflicts
    let suffix = Ulid::new().to_string().to_lowercase();
    let network_name = format!("tc-host-port-net-{suffix}");
    let first_container_name = format!("host-port-first-{suffix}");
    let second_container_name = format!("host-port-second-{suffix}");

    // Create two containers, each with only one host port exposed
    let first_container =
        create_test_container(&first_container_name, &network_name, first_host_port).await?;
    let second_container =
        create_test_container(&second_container_name, &network_name, second_host_port).await?;

    // Test port isolation: each container should only access its own exposed port
    assert_can_access(&first_container, first_host_port, "first-host-service").await?;
    assert_cannot_access(&first_container, second_host_port).await?;
    assert_can_access(&second_container, second_host_port, "second-host-service").await?;
    assert_cannot_access(&second_container, first_host_port).await?;

    // Verify containers can still communicate with each other normally
    let mut first_to_second = first_container
        .exec(ping_once(&second_container_name))
        .await?;
    let _ = first_to_second.stdout_to_vec().await?;
    assert_eq!(first_to_second.exit_code().await?, Some(0));

    let mut second_to_first = second_container
        .exec(ping_once(&first_container_name))
        .await?;
    let _ = second_to_first.stdout_to_vec().await?;
    assert_eq!(second_to_first.exit_code().await?, Some(0));

    Ok(())
}
