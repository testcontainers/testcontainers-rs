# Networking

Testcontainers for Rust includes several helpers to connect containers with the outside world. This page covers two complementary workflows:
- mapping container ports to the host so your test code can reach services inside the container;
- exposing host ports inside a container so the container can call back into services running on your machine.

## Exposing Container Ports to the Host

Calling `with_exposed_port` on an image request asks the container runtime to publish that container port on a random free port on the host (similar to `docker run -p`). This is the most common way to make a containerised service available to your tests.

```rust
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage,
};

#[tokio::test]
async fn talks_to_web_service() -> Result<(), Box<dyn std::error::Error>> {
    let container = GenericImage::new("ghcr.io/testcontainers/examples", "web")
        .with_exposed_port(8080.tcp())
        .with_wait_for(WaitFor::message_on_stdout("listening on 0.0.0.0:8080"))
        .start()
        .await?;

    let host = container.get_host()?;
    let port = container.get_host_port_ipv4(8080.tcp())?;

    let body = reqwest::get(format!("http://{host}:{port}/health"))
        .await?
        .text()
        .await?;
    assert_eq!(body, "OK");

    Ok(())
}
```

The mapped host port is assigned dynamically. Retrieve it using `get_host_port_ipv4` or `get_host_port_ipv6`, depending on the protocol you need. If you must pin a specific host port, switch to `with_mapped_port(host_port, container_port)` instead of `with_exposed_port`.

## Exposing Host Ports to a Container

Some integration tests require containers to reach services running directly on the host. Enabling the optional `host-port-exposure` feature spins up a lightweight SSH sidecar and creates a reverse tunnel for each host port you request.

### Enable the Feature

Declare the feature flag in `Cargo.toml`:

```toml
# Cargo.toml
testcontainers = { version = "*", features = ["host-port-exposure"] }
```

It can be combined with other common crate features. Host port exposure does not support `reusable-containers`, preventing tunnels from leaking across test sessions.

### Request Host Port Access

Use `with_exposed_host_port` or `with_exposed_host_ports` to request access to host ports that are already open. Testcontainers automatically injects the `host.testcontainers.internal` alias into the container and points it at the tunnel.

```rust
use std::io::Write;
use std::net::TcpListener;
use std::thread;

use testcontainers::{
    core::{ExecCommand, IntoContainerPort},
    runners::AsyncRunner,
    GenericImage,
};

const HOST_ALIAS: &str = "host.testcontainers.internal";

#[tokio::test]
async fn container_reaches_host_service() -> Result<(), Box<dyn std::error::Error>> {
    // Start a temporary HTTP server on the host.
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let host_port = listener.local_addr()?.port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\nhello-from-host",
            );
        }
    });

    let container = GenericImage::new("alpine", "3.19")
        .with_entrypoint("/bin/sh")
        .with_cmd(["-c", "sleep 30"])
        .with_exposed_host_port(host_port)
        .start()
        .await?;

    // Verify that the container can reach the host.
    let mut exec = container
        .exec(ExecCommand::new(vec![
            "wget".into(),
            "-qO-".into(),
            format!("http://{HOST_ALIAS}:{host_port}"),
        ]))
        .await?;
    let output = String::from_utf8(exec.stdout_to_vec().await?)?;
    assert_eq!(output, "hello-from-host");

    Ok(())
}
```

All requested ports share the `host.testcontainers.internal` alias. Call `with_exposed_host_ports([port_a, port_b])` to expose multiple services at once; each container keeps its tunnels scoped to itself.

### Usage Notes

- Requested ports must be greater than zero and cannot include `22`, which is reserved for the SSH sidecar.
- Do not predefine the `host.testcontainers.internal` host entry; the feature manages it automatically.
- Host port exposure does not support Docker's `host` network mode or `container:<id>` network sharing.
- Reusable containers are rejected to avoid tunnel conflicts between test runs.

With these tools you can validate both inbound and outbound network flows without leaving the comfort of your Rust test suite.
