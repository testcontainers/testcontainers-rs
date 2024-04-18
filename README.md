# Testcontainers-rs

![Continuous Integration](https://github.com/testcontainers/testcontainers-rs/workflows/Continuous%20Integration/badge.svg?branch=dev)
[![Crates.io](https://img.shields.io/crates/v/testcontainers.svg)](https://crates.io/crates/testcontainers)
[![Docs.rs](https://docs.rs/testcontainers/badge.svg)](https://docs.rs/testcontainers)
[![Slack](https://img.shields.io/badge/Slack-join-orange?style=flat&logo=slack&)](https://join.slack.com/t/testcontainers/shared_invite/zt-2gra37tid-n9xDJGjjVb7hMRanGjowkw)

Testcontainers-rs is the official Rust language fork of [http://testcontainers.org](http://testcontainers.org).

## Usage

### `testcontainers` is the core crate

The crate provides an API for working with containers in a test environment.

1. Depend on `testcontainers`
2. Implement `testcontainers::core::Image` for necessary docker-images
3. Run it with any available runner `testcontainers::core::runners::*` (use `blocking` feature for blocking API)

#### Example:

- Blocking API (under `blocking` feature)
```rust
use testcontainers::{
    GenericImage,
    core::{WaitFor, runners::SyncRunner}
};

#[test]
fn test_redis() {
    let container = GenericImage::new("redis", "latest")
        .with_exposed_port(6379)
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start();
}
```

- Async API
```rust
use testcontainers::{
    GenericImage,
    core::{WaitFor, runners::AsyncRunner}
};

#[tokio::test]
async fn test_redis() {
    let container = GenericImage::new("redis", "latest")
        .with_exposed_port(6379)
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()
        .await;
}
```

### Ready-to-use images

The easiest way to use `testcontainers` is to depend on ready-to-use images (aka modules).

Modules are available as a community-maintained crate: [testcontainers-modules](https://github.com/testcontainers/testcontainers-rs-modules-community)

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-Apache-2.0) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
