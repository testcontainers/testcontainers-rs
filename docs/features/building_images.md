# Building Docker Images

Testcontainers for Rust supports building Docker images directly within your tests. This is useful when you need to test against custom-built images with specific configurations, or when your test requires a dynamically generated Dockerfile.

## Building a Simple Image

Use [`GenericBuildableImage`](https://docs.rs/testcontainers/latest/testcontainers/struct.GenericBuildableImage.html) to define an image that will be built from a Dockerfile:

```rust
use testcontainers::{
    core::WaitFor,
    runners::AsyncRunner,
    GenericBuildableImage,
};

#[tokio::test]
async fn test_custom_image() -> Result<(), Box<dyn std::error::Error>> {
    let image = GenericBuildableImage::new("my-test-app", "latest")
        .with_dockerfile_string(r#"
            FROM alpine:latest
            COPY --chmod=0755 app.sh /usr/local/bin/app
            ENTRYPOINT ["/usr/local/bin/app"]
        "#)
        .with_data(
            r#"#!/bin/sh
echo "Hello from custom image!"
"#,
            "./app.sh",
        )
        .build_image()
        .await?;

    let container = image
        .with_wait_for(WaitFor::message_on_stdout("Hello from custom image!"))
        .start()
        .await?;

    Ok(())
}
```

## Adding Files to the Build Context

You can add files from your filesystem or provide inline data:

### From Filesystem

```rust
let image = GenericBuildableImage::new("my-app", "latest")
    .with_dockerfile("./path/to/Dockerfile")
    .with_file("./target/release/myapp", "./myapp")
    .build_image()
    .await?;
```

### Inline Data

```rust
let image = GenericBuildableImage::new("my-app", "latest")
    .with_dockerfile_string("FROM alpine:latest\nCOPY config.json /config.json")
    .with_data(r#"{"port": 8080}"#, "./config.json")
    .build_image()
    .await?;
```

## Build Options

The [`BuildImageOptions`](https://docs.rs/testcontainers/latest/testcontainers/core/struct.BuildImageOptions.html) struct provides fine-grained control over the build process.

### Skip Building if Image Exists

When running tests repeatedly, you can skip rebuilding if the image already exists:

```rust
use testcontainers::core::BuildImageOptions;

let image = GenericBuildableImage::new("my-app", "v1.0")
    .with_dockerfile_string("FROM alpine:latest")
    .build_image_with(
        BuildImageOptions::new()
            .with_skip_if_exists(true)
    )
    .await?;
```

This option:
- Checks if an image with the same descriptor (name:tag) already exists
- Skips the build if found, using the existing image
- Is thread-safe - parallel tests building the same image will be serialized

### Disable Build Cache

Force a fresh build without using Docker's layer cache:

```rust
let image = GenericBuildableImage::new("my-app", "latest")
    .with_dockerfile_string("FROM alpine:latest\nRUN apk update")
    .build_image_with(
        BuildImageOptions::new()
            .with_no_cache(true)
    )
    .await?;
```

### Build Arguments

Pass build-time variables to your Dockerfile using `ARG` instructions:

```rust
let image = GenericBuildableImage::new("my-app", "latest")
    .with_dockerfile_string(r#"
        FROM alpine:latest
        ARG VERSION
        ARG BUILD_DATE
        RUN echo "Building version ${VERSION} on ${BUILD_DATE}"
    "#)
    .build_image_with(
        BuildImageOptions::new()
            .with_build_arg("VERSION", "1.0.0")
            .with_build_arg("BUILD_DATE", "2024-10-25")
    )
    .await?;
```

You can also provide build arguments as a HashMap:

```rust
use std::collections::HashMap;

let mut args = HashMap::new();
args.insert("VERSION".to_string(), "1.0.0".to_string());
args.insert("ENVIRONMENT".to_string(), "test".to_string());

let image = GenericBuildableImage::new("my-app", "latest")
    .with_dockerfile_string("FROM alpine:latest\nARG VERSION\nARG ENVIRONMENT")
    .build_image_with(
        BuildImageOptions::new()
            .with_build_args(args)
    )
    .await?;
```

### Combining Options

All build options can be chained together:

```rust
let image = GenericBuildableImage::new("my-app", "latest")
    .with_dockerfile_string("FROM alpine:latest\nARG VERSION")
    .build_image_with(
        BuildImageOptions::new()
            .with_skip_if_exists(true)
            .with_no_cache(false)
            .with_build_arg("VERSION", "1.0.0")
    )
    .await?;
```

## Synchronous API

For non-async tests, use the [`SyncRunner`](https://docs.rs/testcontainers/latest/testcontainers/runners/trait.SyncRunner.html) trait (requires the `blocking` feature):

```rust
use testcontainers::{
    runners::SyncRunner,
    GenericBuildableImage,
};

#[test]
fn test_sync_build() -> Result<(), Box<dyn std::error::Error>> {
    let image = GenericBuildableImage::new("my-app", "latest")
        .with_dockerfile_string("FROM alpine:latest")
        .build_image()?;

    let container = image.start()?;
    Ok(())
}
```

## Best Practices

### Use Descriptive Tags

Use meaningful image names and tags to avoid conflicts:

```rust
// Good: specific and descriptive
GenericBuildableImage::new("test-user-service", "integration-v1")

// Avoid: generic names that might conflict
GenericBuildableImage::new("test", "latest")
```

### Leverage skip_if_exists for Faster Tests

When your image doesn't change between test runs:

```rust
let image = GenericBuildableImage::new("my-stable-app", "v1.0")
    .with_dockerfile_string("FROM alpine:latest\nRUN apk add curl")
    .build_image_with(
        BuildImageOptions::new()
            .with_skip_if_exists(true)
    )
    .await?;
```

### Use Build Arguments for Flexibility

Make your test images configurable:

```rust
fn build_test_image(version: &str) -> GenericBuildableImage {
    GenericBuildableImage::new("my-app", version)
        .with_dockerfile_string("FROM alpine:latest\nARG APP_VERSION\nENV VERSION=$APP_VERSION")
        .build_image_with(
            BuildImageOptions::new()
                .with_build_arg("APP_VERSION", version)
        )
}
```

## Common Patterns

### Building from a Complex Dockerfile

```rust
let dockerfile = r#"
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/myapp /usr/local/bin/
CMD ["/usr/local/bin/myapp"]
"#;

let image = GenericBuildableImage::new("my-rust-app", "latest")
    .with_dockerfile_string(dockerfile)
    .with_file("./Cargo.toml", "./Cargo.toml")
    .with_file("./Cargo.lock", "./Cargo.lock")
    .with_file("./src", "./src")
    .build_image()
    .await?;
```

### Building Multiple Variants

```rust
async fn build_variant(variant: &str, port: u16) -> Result<GenericImage, Box<dyn std::error::Error>> {
    GenericBuildableImage::new("my-app", variant)
        .with_dockerfile_string(format!(r#"
            FROM alpine:latest
            ARG PORT
            ENV APP_PORT=$PORT
            CMD ["sh", "-c", "echo 'Running on port $APP_PORT' && sleep infinity"]
        "#))
        .build_image_with(
            BuildImageOptions::new()
                .with_build_arg("PORT", port.to_string())
                .with_skip_if_exists(true)
        )
        .await
}

#[tokio::test]
async fn test_multiple_variants() -> Result<(), Box<dyn std::error::Error>> {
    let image1 = build_variant("dev", 8080).await?;
    let image2 = build_variant("staging", 8081).await?;

    // Use images in tests...
    Ok(())
}
```

## Troubleshooting

### Build Fails with "no active session" Error

This typically occurs when BuildKit encounters issues. Potential solutions:

1. Remove the build cache and retry:
   ```rust
   BuildImageOptions::new().with_no_cache(true)
   ```

2. Ensure Docker daemon is running properly:
   ```bash
   docker info
   ```

3. In CI environments, ensure BuildKit is properly initialized

### Image Not Found After Build

Verify the descriptor matches exactly:

```rust
let image = GenericBuildableImage::new("my-app", "v1.0")  // Name and tag must match
    .with_dockerfile_string("FROM alpine:latest")
    .build_image()
    .await?;

// The image is now available as "my-app:v1.0"
```

### Build Arguments Not Working

Ensure ARG instructions are in the Dockerfile before they're used:

```dockerfile
# Correct
ARG VERSION
ENV APP_VERSION=$VERSION

# Incorrect - ARG comes after usage
ENV APP_VERSION=$VERSION
ARG VERSION
```
