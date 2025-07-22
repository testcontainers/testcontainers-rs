#![cfg(feature = "blocking")]

use std::net::{Ipv6Addr, TcpListener};

use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::{SyncBuilder, SyncRunner},
    GenericBuildableImage,
};

/// Test the functionality of exposing container ports over both IPv4 and IPv6.
#[test]
fn test_ipv4_ipv6_host_ports() -> anyhow::Result<()> {
    let _ = pretty_env_logger::try_init();

    let image = GenericBuildableImage::new("simple_web_server", "latest")
        // "Dockerfile" is included already, so adding the build context directory is all what is needed
        .with_file(
            std::fs::canonicalize("../testimages/simple_web_server").unwrap(),
            ".",
        )
        .build_image()
        .unwrap()
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .with_wait_for(WaitFor::seconds(1));

    // Run one container, and check what ephemeral ports it uses. Perform test HTTP requests to
    // both bound ports.
    let first_container = image.clone().start()?;
    let first_ipv4_port = first_container.get_host_port_ipv4(80.tcp())?;
    let first_ipv6_port = first_container.get_host_port_ipv6(80.tcp())?;
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://127.0.0.1:{first_ipv4_port}"))?.text()?,
    );
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://[::1]:{first_ipv6_port}"))?.text()?,
    );

    // Bind to several subsequent ports in the ephemeral range, only on IPv6. This should cause
    // Docker's IPv4 and IPv6 port allocation to no longer be in lock step, (if they were before)
    // as the IPv6 allocator would have to skip the ports we grabbed.
    let mut sockets = Vec::new();
    for port in first_ipv6_port + 1..first_ipv6_port + 9 {
        if let Ok(socket) = TcpListener::bind((Ipv6Addr::LOCALHOST, port)) {
            sockets.push(socket);
        }
    }

    // Run a second container, and repeat test HTTP requests with it. This confirms that handling
    // of both IPv4 and IPv6 host port bindings is correct, because at this point,
    // `second_ipv4_port` and `second_ipv6_port` are very unlikely to be the same.
    let second_container = image.start()?;
    let second_ipv4_port = second_container.get_host_port_ipv4(80.tcp())?;
    let second_ipv6_port = second_container.get_host_port_ipv6(80.tcp())?;
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://127.0.0.1:{second_ipv4_port}"))?.text()?,
    );
    assert_eq!(
        "foo",
        reqwest::blocking::get(format!("http://[::1]:{second_ipv6_port}"))?.text()?,
    );
    Ok(())
}
