use testcontainers::{
    core::{ExecCommand, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
use tokio::net::TcpListener;

/// This example demonstrates host port exposure functionality with actual connectivity testing.
///
/// Run with RUST_LOG=trace to see detailed platform detection and host mapping logs:
/// RUST_LOG=trace cargo run --example host_port_exposure_demo
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    pretty_env_logger::init();

    println!("🔍 Testing host port exposure with actual host service...");

    // Start a simple TCP server on the host to demonstrate connectivity
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let host_port = listener.local_addr()?.port();
    println!("🚀 Started demo service on host port {}", host_port);

    // Accept connections in background
    let listener_handle = tokio::spawn(async move {
        while let Ok((mut stream, addr)) = listener.accept().await {
            println!("📞 Host service: received connection from {}", addr);
            use tokio::io::AsyncWriteExt;
            let response =
                "HTTP/1.1 200 OK\r\nContent-Length: 25\r\n\r\nHello from host service!\n";
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.flush().await;
        }
    });

    // Create a container that exposes the host port
    let container = GenericImage::new("alpine", "latest")
        .with_wait_for(WaitFor::seconds(1))
        .with_exposed_host_port(host_port)
        .with_cmd(["sleep", "30"])
        .start()
        .await?;

    println!("✅ Container started with host port {} exposed!", host_port);
    println!(
        "📋 The container can access the host service via host.testcontainers.internal:{}",
        host_port
    );

    // Wait for the container to be ready
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Check /etc/hosts configuration inside the container
    let mut exec_result = container
        .exec(ExecCommand::new(["cat", "/etc/hosts"]))
        .await?;

    let stdout = exec_result.stdout_to_vec().await?;
    let hosts_content = String::from_utf8_lossy(&stdout);
    println!("\n📝 Container /etc/hosts content:");
    for line in hosts_content.lines() {
        if line.contains("host.testcontainers.internal") {
            println!("   ✓ {}", line);
        } else if !line.trim().is_empty() {
            println!("     {}", line);
        }
    }

    // Verify the mapping exists
    if hosts_content.contains("host.testcontainers.internal") {
        println!("✅ host.testcontainers.internal is configured in /etc/hosts");
    } else {
        println!("❌ host.testcontainers.internal not found in /etc/hosts");
        container.stop().await?;
        listener_handle.abort();
        return Ok(());
    }

    // Test connectivity from container to host service
    println!("\n🔗 Testing connectivity from container to host service...");
    let mut exec_result = container
        .exec(ExecCommand::new([
            "sh",
            "-c",
            &format!(
                "echo 'GET / HTTP/1.1\\r\\nHost: host.testcontainers.internal\\r\\n\\r\\n' | nc host.testcontainers.internal {} && echo 'Connection successful' || echo 'Connection failed'",
                host_port
            )
        ]))
        .await?;

    let stdout = exec_result.stdout_to_vec().await?;
    let output = String::from_utf8_lossy(&stdout);

    if output.contains("Hello from host service!") {
        println!("✅ Successfully connected to host service from container!");
        println!("📄 Response received: \"Hello from host service!\"");
    } else if output.contains("Connection failed") {
        println!("❌ Failed to connect to host service");
    } else {
        println!("⚠️  Unexpected output: {}", output.trim());
    }

    println!("\n🛑 Stopping container...");
    container.stop().await?;
    listener_handle.abort();

    println!("✨ Demo completed!");
    Ok(())
}
