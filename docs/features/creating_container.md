# Creating containers

Build a container request by chaining `ImageExt` methods on an image, then start it with
`AsyncRunner` or `SyncRunner`.

```rust
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

let _container = GenericImage::new("redis", "7.2.4")
    .with_exposed_port(6379.tcp())
    .with_env_var("DEBUG", "1")
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .start()
    .await
    .unwrap();
```

## Common settings

- Ports and networking: use `with_exposed_port`, `with_mapped_port`, or `with_network`
  (see [networking](networking.md)).
- Files and mounts: use `with_copy_to` and `with_mount` (see [files](files.md)).
- Readiness: use `with_wait_for` or health checks (see [wait strategies](wait_strategies.md)).
- Execution hooks: use `exec_after_start` or `exec_before_ready` on images
  (see [exec commands](exec_commands.md)).

## Custom images

If you want a strongly-typed image with reusable defaults, define your own type and implement
`Image` for it, then use it like any other image (for example, `RedisImage` or `PostgresImage`).
Before rolling your own, check the community modules that already package popular services:
[`community modules`][community-modules] and the
[`testcontainers-rs-modules-community` repo][community-modules-repo].

```rust
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    Image,
};

#[derive(Debug, Clone)]
struct RedisImage {
    ports: [ContainerPort; 1],
}

impl Default for RedisImage {
    fn default() -> Self {
        Self {
            ports: [ContainerPort::Tcp(6379)],
        }
    }
}

impl Image for RedisImage {
    fn name(&self) -> &str {
        "redis"
    }

    fn tag(&self) -> &str {
        "7.2.4"
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &self.ports
    }
}

let _container = RedisImage::default().start().await.unwrap();
```

## Advanced settings

If you need to tweak Docker `HostConfig` fields that are not exposed by the high-level API, use
`ImageExt::with_host_config_modifier` to apply a single callback just before container creation.
The modifier runs after `testcontainers` fills in its defaults. If you call it multiple times,
the last modifier wins.

```rust
use testcontainers::{GenericImage, ImageExt};

let image = GenericImage::new("testcontainers/helloworld", "1.3.0")
    .with_host_config_modifier(|host_config| {
        host_config.cpu_period = Some(100_000);
        host_config.cpu_quota = Some(200_000);
    });
```

[community-modules]: ../quickstart/community_modules.md
[community-modules-repo]: https://github.com/testcontainers/testcontainers-rs-modules-community
