# Docker Compose Support

Testcontainers for Rust supports running multi-container applications defined in Docker Compose files. This is useful when your tests need multiple interconnected services or when you want to reuse existing docker-compose configurations from your development environment.

> **Note:** Docker Compose support is currently only available for async runtimes. Synchronous/blocking support may be added in a future release.

## Installation

Add the `docker-compose` feature to your dependencies:

```toml
[dev-dependencies]
testcontainers = { version = "x.y.z", features = ["docker-compose"] }
```

## Minimal Example

```rust
use testcontainers::compose::DockerCompose;

#[tokio::test]
async fn test_redis() -> Result<(), Box<dyn std::error::Error>> {
    let mut compose = DockerCompose::with_local_client(&["tests/docker-compose.yml"]);
    compose.up().await?;

    let redis = compose.service("redis").expect("redis service");
    let port = redis.get_host_port_ipv4(6379).await?;

    // Use redis at localhost:{port}
    let client = redis::Client::open(format!("redis://localhost:{}", port))?;
    let mut con = client.get_connection()?;
    redis::cmd("PING").query::<String>(&mut con)?;

    Ok(())
}
```

With `docker-compose.yml`:

```yaml
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379"
```

## Basic Usage

Use [`DockerCompose`](https://docs.rs/testcontainers/latest/testcontainers/compose/struct.DockerCompose.html) to start services defined in your compose files:

```rust
use testcontainers::compose::DockerCompose;

#[tokio::test]
async fn test_with_compose() -> Result<(), Box<dyn std::error::Error>> {
    let mut compose = DockerCompose::with_local_client(&["tests/docker-compose.yml"]);

    compose.up().await?;

    // Access service by name
    let web = compose.service("web").expect("web service");
    let port = web.get_host_port_ipv4(8080).await?;

    let response = reqwest::get(format!("http://localhost:{}", port)).await?;
    assert!(response.status().is_success());

    Ok(())
    // Automatic cleanup on drop
}
```

## Accessing Services

After calling `up()`, you can access individual services by name. The `service()` method returns a reference to the container, providing full access to the container API:

### Get Service Container

```rust
compose.up().await?;

let redis = compose.service("redis").expect("redis service exists");

// Full container API available
let port = redis.get_host_port_ipv4(6379).await?;
let logs = redis.stdout(true);
redis.exec(ExecCommand::new(["redis-cli", "PING"])).await?;
```

### List All Services

```rust
compose.up().await?;

for service_name in compose.services() {
    println!("Service: {}", service_name);
}
```

### Access Ports, Logs, and Execute Commands

Services return a container reference with the full API:

```rust
compose.up().await?;

let redis = compose.service("redis").expect("redis service");

// Get mapped ports
let port = redis.get_host_port_ipv4(6379).await?;
let ipv6_port = redis.get_host_port_ipv6(6379).await?;

// Stream logs
let stdout = redis.stdout(true);
let stderr = redis.stderr(false);

// Execute commands
let result = redis.exec(ExecCommand::new(["redis-cli", "PING"])).await?;

// Get container info
let container_id = redis.id();
let host = redis.get_host().await?;
```

## Client Modes

### Local Client (Default)

Uses the locally installed `docker compose` CLI:

```rust
let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"]);
compose.up().await?;
```

**Requirements:**
- Docker CLI with Compose plugin installed locally
- Compose files must be on the filesystem

### Containerised Client

Runs `docker compose` inside a container (no local Docker CLI required):

```rust
let mut compose = DockerCompose::with_containerised_client(&["docker-compose.yml"]).await;
compose.up().await?;
```

**Benefits:**
- No local Docker CLI installation needed
- Consistent compose version across environments
- Useful for CI/CD where Docker CLI might not be available

## Configuration Options

### Environment Variables

Pass environment variables to your compose stack:

```rust
use std::collections::HashMap;

let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"])
    .with_env("DATABASE_URL", "postgres://test:test@db:5432/test")
    .with_env("REDIS_PORT", "6380");

compose.up().await?;
```

Or use a HashMap for bulk configuration:

```rust
let mut env_vars = HashMap::new();
env_vars.insert("API_KEY".to_string(), "test-key-123".to_string());
env_vars.insert("DEBUG".to_string(), "true".to_string());

let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"])
    .with_env_vars(env_vars);

compose.up().await?;
```

### Lifecycle and Cleanup

You can either let the stack automatically clean up on drop, or explicitly tear it down:

```rust
// Option 1: Automatic cleanup (default behavior)
{
    let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"]);
    compose.up().await?;
    // Use services...
} // Automatically cleaned up here

// Option 2: Explicit teardown
let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"]);
compose.up().await?;
// Use services...
compose.down().await?; // Explicit cleanup, consumes compose
```

Control what gets removed during cleanup:

```rust
let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"]);

// Remove volumes on cleanup (default: true)
compose.with_remove_volumes(true);

// Remove images on cleanup (default: false)
compose.with_remove_images(false);

compose.up().await?;
```

### Build and Pull Options

Configure whether to build or pull images before starting:

```rust
let mut compose = DockerCompose::with_local_client(&["docker-compose.yml"])
    .with_build(true)   // Build images defined in compose file
    .with_pull(true);   // Pull latest images before starting

compose.up().await?;
```

## Multiple Compose Files

You can use multiple compose files (e.g., base + override):

```rust
let mut compose = DockerCompose::with_local_client(&[
    "docker-compose.yml",
    "docker-compose.test.yml",
]);

compose.up().await?;
```

## Complete Example

```rust
use testcontainers::{
    compose::DockerCompose,
    core::IntoContainerPort,
};

#[tokio::test]
async fn integration_test_with_compose() -> Result<(), Box<dyn std::error::Error>> {
    let mut compose = DockerCompose::with_local_client(&[
        "tests/docker-compose.yml",
    ])
    .with_env("POSTGRES_PASSWORD", "test-password")
    .with_env("REDIS_MAXMEMORY", "256mb");

    compose.up().await?;

    // List all running services
    println!("Running services: {:?}", compose.services());

    // Access database
    let db_port = compose.get_host_port_ipv4("db", 5432).await?;
    let db_url = format!("postgres://postgres:test-password@localhost:{}/test", db_port);
    let db_pool = sqlx::PgPool::connect(&db_url).await?;

    // Run migrations or setup
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id INT PRIMARY KEY)")
        .execute(&db_pool)
        .await?;

    // Access Redis
    let redis_port = compose.get_host_port_ipv4("redis", 6379).await?;
    let redis_client = redis::Client::open(format!("redis://localhost:{}", redis_port))?;
    let mut con = redis_client.get_connection()?;
    redis::cmd("SET").arg("test-key").arg("test-value").query::<()>(&mut con)?;

    // Access web service
    let web_port = compose.get_host_port_ipv4("web", 8080).await?;
    let response = reqwest::get(format!("http://localhost:{}/health", web_port))
        .await?
        .text()
        .await?;

    assert_eq!(response, "OK");

    Ok(())
    // Automatic cleanup: containers, networks, and volumes are removed
}
```

## Sample Compose File

Here's an example `docker-compose.yml` that works with the above code:

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: test
    ports:
      - "5432"

  redis:
    image: redis:7-alpine
    command: redis-server --maxmemory ${REDIS_MAXMEMORY:-128mb}
    ports:
      - "6379"

  web:
    image: my-web-app:latest
    environment:
      DATABASE_URL: postgres://postgres:${POSTGRES_PASSWORD}@db:5432/test
      REDIS_URL: redis://redis:6379
    ports:
      - "8080"
    depends_on:
      - db
      - redis
```

## Best Practices

### Use Unique Project Names

Each test gets a unique project name automatically via UUID, preventing conflicts between parallel tests. No manual configuration needed.

### Rely on Compose's --wait Flag

The compose `up()` method uses Docker Compose's built-in `--wait` functionality, which waits for services to be healthy before returning. Configure healthchecks in your compose file:

```yaml
services:
  web:
    image: nginx
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost"]
      interval: 5s
      timeout: 3s
      retries: 3
    ports:
      - "80"
```

### Clean Up Resources

By default, volumes are removed on cleanup but images are not. Adjust based on your needs:

```rust
// Keep volumes for debugging or to reuse data across test runs
compose.with_remove_volumes(false);

// Remove images to save disk space
compose.with_remove_images(true);
```

## Troubleshooting

### Service Not Found

If `compose.service("name")` returns `None`:

1. Check the service name matches exactly what's in your compose file
2. Ensure `up()` was called and succeeded
3. Verify the service started successfully (check Docker logs)

### Port Not Exposed Error

If `get_host_port_ipv4()` fails:

1. Ensure the port is listed in the `ports:` section of your compose file
2. Use the **container's internal port**, not the host port
3. Example: If mapped as `"8081:80"`, use `compose.get_host_port_ipv4("web", 80)`

### Compose Command Fails

If `up()` returns an error:

1. Verify Docker Compose is installed: `docker compose version`
2. Check compose file is valid: `docker compose -f your-file.yml config`
3. Ensure all required images are available or can be pulled

