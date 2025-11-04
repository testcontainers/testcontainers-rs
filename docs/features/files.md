# Files and Mounts

Rust Testcontainers lets you seed container filesystems before startup, collect artifacts produced inside containers, and bind host paths at runtime. The APIs deliver smooth ergonomics while staying idiomatic to Rust.

## Copying Files Into Containers (Before Startup)

Use `ImageExt::with_copy_to` to stage files or directories before the container starts. Content can come from raw bytes or host paths:

```rust
// Example: copying inline bytes and directories into a container
use testcontainers::{GenericImage, WaitFor};

let project_assets = std::path::Path::new("tests/fixtures/assets");
let image = GenericImage::new("alpine", "latest")
    .with_wait_for(WaitFor::seconds(1))
    .with_copy_to("/opt/app/config.yaml", br#"mode = "test""#.to_vec())
    .with_copy_to("/opt/app/assets", project_assets);
```

Everything is packed into a TAR archive, preserving nested directories. The helper accepts either `Vec<u8>` or any path-like value implementing `CopyDataSource`.  
Note: file permissions and symbolic links follow Docker’s default TAR handling.

## Copying Files From Containers (After Execution)

Use `copy_file_from` to pull data produced inside the container:

```rust
// Example: copying a file from a running container to the host
use tempfile::tempdir;
use testcontainers::{GenericImage, WaitFor};

#[tokio::test]
async fn copy_example() -> anyhow::Result<()> {
    let container = GenericImage::new("alpine", "latest")
        .with_cmd(["sh", "-c", "echo '42' > /tmp/result.txt && sleep 10"])
        .with_wait_for(WaitFor::seconds(1))
        .start()
        .await?;

    let destination = tempdir()?.path().join("result.txt");
    container
        .copy_file_from("/tmp/result.txt", destination.as_path())
        .await?;
    assert_eq!(tokio::fs::read_to_string(&destination).await?, "42\n");
    Ok(())
}
```

- `copy_file_from` streams the sole regular-file entry produced by Docker into any destination implementing `CopyFileFromContainer` (for example `&Path`, `PathBuf`, `Vec<u8>`, or `&mut Vec<u8>`).  
  It verifies that **exactly one** file exists and returns an error (e.g., `CopyFileError::UnexpectedDirectory`) when the path resolves to a directory or an unsupported TAR record.
- To capture the contents in memory:
  ```rust
  let mut bytes = Vec::new();
  container.copy_file_from("/tmp/result.txt", &mut bytes).await?;
  ```

The blocking `Container` type provides the same `copy_file_from` API.

## Using Mounts for Writable Workspaces

When a bind or tmpfs mount fits better than copy semantics, use the `Mount` helpers:

```rust
// Example: mounting a host directory for read/write access
use std::path::Path;
use testcontainers::core::{mounts::Mount, AccessMode, MountType};

let host_data = Path::new("/var/tmp/integration-data");
let mount = Mount::bind(host_data, "/workspace")
    .with_mode(AccessMode::ReadWrite)
    .with_type(MountType::Bind);

let image = GenericImage::new("python", "3.13")
    .with_mount(mount)
    .with_cmd(["python", "/workspace/run.py"]);
```

Bind mounts share host state directly. Tmpfs mounts create ephemeral in-memory storage useful for scratch data or caches.

## Selecting an Approach

- **Copy before startup** — for deterministic inputs.
- **Copy from containers** — to capture build artifacts, logs, or test fixtures produced during a run.
- **Use mounts** — when containers need to read/write large amounts of data efficiently without re-tarring.

Mixing these tools keeps tests hermetic (isolated and reproducible) while letting you inspect outputs locally.  
Document each choice in code so teammates know whether data is ephemeral (`tmpfs`), seeded once (`with_copy_to`), or captured for later assertions (`copy_file_from`).