use testcontainers::{
    core::{ExecCommand, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
use tokio::{io::AsyncWriteExt, net::TcpListener};

// Helper functions for reusable test logic

/// Creates a TCP server that handles unlimited connections with the same response
async fn create_host_service() -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let host_port = listener.local_addr()?.port();

    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    println!("Host service: received connection from {}", addr);
                    let _ = stream.write_all(b"Hello from host service!").await;
                    let _ = stream.flush().await;
                }
                Err(e) => {
                    println!("Host service: failed to accept connection: {}", e);
                    break;
                }
            }
        }
    });

    Ok((host_port, handle))
}

/// Creates a container with host port mapping
async fn create_container_with_host_port(
    host_port: u16,
) -> anyhow::Result<testcontainers::ContainerAsync<GenericImage>> {
    GenericImage::new("alpine", "latest")
        .with_wait_for(WaitFor::seconds(1))
        .with_exposed_host_port(host_port)
        .with_cmd(["sleep", "30"])
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start container: {}", e))
}

/// Tests connection from container to host service
async fn test_container_connection(
    container: &testcontainers::ContainerAsync<GenericImage>,
    host_port: u16,
    identifier: Option<usize>,
    timeout: u64,
    wait_time: u64,
) -> anyhow::Result<bool> {
    // Give containers time to start up
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let (success_msg, fail_msg) = if let Some(id) = identifier {
        (
            format!("Container {} connected successfully", id),
            format!("Container {} connection failed", id),
        )
    } else {
        (
            "Connection successful".to_string(),
            "Connection failed".to_string(),
        )
    };

    let mut exec_result = container
        .exec(ExecCommand::new([
            "sh",
            "-c",
            &format!(
                "timeout {} nc -v -w {} host.testcontainers.internal {} < /dev/null && echo '{}' || echo '{}'",
                timeout, wait_time, host_port, success_msg, fail_msg
            )
        ]))
        .await?;

    let stdout = exec_result.stdout_to_vec().await?;
    let output = String::from_utf8_lossy(&stdout);

    if let Some(id) = identifier {
        println!("Container {} output: {}", id, output.trim());
    } else {
        println!("Container connection output: {}", output.trim());
    }

    Ok(output.contains(&success_msg))
}

/// Verifies the /etc/hosts file contains proper host.testcontainers.internal mapping
async fn verify_hosts_mapping(
    container: &testcontainers::ContainerAsync<GenericImage>,
) -> anyhow::Result<()> {
    let mut exec_result = container
        .exec(ExecCommand::new(["cat", "/etc/hosts"]))
        .await?;

    let stdout = exec_result.stdout_to_vec().await?;
    let hosts_content = String::from_utf8_lossy(&stdout);

    for line in hosts_content.lines() {
        if line.contains("host.testcontainers.internal") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let ip = parts[0];
                if ip.parse::<std::net::Ipv4Addr>().is_err()
                    && ip.parse::<std::net::Ipv6Addr>().is_err()
                {
                    anyhow::bail!(
                        "host.testcontainers.internal should map to a valid IP address but found: {}\n/etc/hosts content:\n{}",
                        line,
                        hosts_content
                    );
                }
                return Ok(());
            } else {
                anyhow::bail!(
                    "Invalid hosts entry format for host.testcontainers.internal: {}\n/etc/hosts content:\n{}",
                    line,
                    hosts_content
                );
            }
        }
    }

    anyhow::bail!(
        "host.testcontainers.internal not found in /etc/hosts - host port mapping is not properly configured\n/etc/hosts content:\n{}",
        hosts_content
    );
}

/// Executes multiple container tasks in parallel and collects results
async fn test_parallel_connections(
    container_count: usize,
    host_port: u16,
) -> anyhow::Result<(usize, Vec<String>)> {
    let mut container_tasks = Vec::new();

    for i in 0..container_count {
        let task = tokio::spawn(async move {
            let container = create_container_with_host_port(host_port)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start container {}: {}", i, e))?;

            let success = test_container_connection(&container, host_port, Some(i), 10, 2).await?;

            container.stop().await?;
            Ok::<bool, anyhow::Error>(success)
        });

        container_tasks.push(task);
    }

    // Wait for all containers to complete their connections
    let mut successful_connections = 0;
    let mut errors = Vec::new();

    for (i, task) in container_tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(true)) => {
                successful_connections += 1;
                println!("Container {} successfully connected", i);
            }
            Ok(Ok(false)) => {
                errors.push(format!("Container {} failed to connect", i));
            }
            Ok(Err(e)) => {
                errors.push(format!("Container {} error: {}", i, e));
            }
            Err(e) => {
                errors.push(format!("Container {} task error: {}", i, e));
            }
        }
    }

    Ok((successful_connections, errors))
}

#[tokio::test]
async fn test_exposed_host_port_api() -> anyhow::Result<()> {
    // Test that the API works without errors
    let container_request = GenericImage::new("alpine", "latest")
        .with_exposed_host_port(8080)
        .with_exposed_host_port(5432)
        .with_cmd(["sleep", "5"]);

    // Verify that the host ports are stored correctly
    assert_eq!(container_request.exposed_host_ports(), &[8080, 5432]);

    // Verify the container can be started (this tests the integration)
    let container = container_request.start().await?;

    // Clean up
    container.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_host_port_mapping_in_container() -> anyhow::Result<()> {
    // This test verifies that the container starts successfully with exposed host ports
    // and checks if host.testcontainers.internal is properly configured in /etc/hosts
    let container = create_container_with_host_port(8080).await?;

    let result = verify_hosts_mapping(&container).await;
    container.stop().await?;

    result
}

#[tokio::test]
async fn test_host_service_connection() -> anyhow::Result<()> {
    // Start a simple TCP server on the host to simulate a service
    let (host_port, service_handle) = create_host_service().await?;
    println!("Started host service on port {}", host_port);

    // Start a container with host port mapping
    let container = create_container_with_host_port(host_port).await?;

    // Test connection from container to host service
    let success = test_container_connection(&container, host_port, None, 5, 1).await?;

    // Cleanup
    container.stop().await?;
    service_handle.abort();

    if success {
        println!("Container successfully connected to host service");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Container failed to connect to host service on port {}",
            host_port
        ))
    }
}

#[tokio::test]
async fn test_parallel_host_service_connections() -> anyhow::Result<()> {
    // Start a host service that handles unlimited connections
    let container_count = 5;
    let (host_port, service_handle) = create_host_service().await?;
    println!(
        "Started host service on port {} for parallel test",
        host_port
    );

    // Execute parallel container tasks
    let (successful_connections, errors) =
        test_parallel_connections(container_count, host_port).await?;

    // Clean up service
    service_handle.abort();

    // Check results
    println!(
        "Parallel test results: {}/{} containers connected successfully",
        successful_connections, container_count
    );

    if !errors.is_empty() {
        println!("Errors encountered:");
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // Verify that most containers connected successfully
    if successful_connections < container_count - 1 {
        anyhow::bail!(
            "Too many connection failures: only {}/{} containers connected successfully. Errors: {:?}",
            successful_connections,
            container_count,
            errors
        );
    }

    println!("Parallel host service connection test passed!");
    Ok(())
}
