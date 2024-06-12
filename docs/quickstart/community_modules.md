_Testcontainers for Rust_ are provided as two separate crates: `testcontainers` and `testcontainers-modules`.

While `testcontainers` is the core crate that provides an API for working with containers in a test environment,
`testcontainers-modules` is a community-maintained crate that provides ready-to-use images (aka modules).

Usually, it's easier to depend on ready-to-use images, as it saves time and effort.
This guide will show you how to use it.

## 1. Usage

1. Depend on [testcontainers-modules] with necessary features (e.g `postgres`, `minio` etc.)
    - Enable `blocking` feature if you want to use modules
      within synchronous tests (feature-gate for `SyncRunner`)
2. Then start using the modules inside your tests with either `AsyncRunner` or `SyncRunner`

Simple example of using `postgres` module with `SyncRunner` (`blocking` and `posrges` features enabled):

```rust
use testcontainers_modules::{postgres, testcontainers::runners::SyncRunner};

#[test]
fn test_with_postgres() {
    let container = postgres::Postgres::default().start().unwrap();
    let host_port = container.get_host_port_ipv4(5432).unwrap();
    let connection_string = &format!(
        "postgres://postgres:postgres@127.0.0.1:{host_port}/postgres",
    );
}
```

> You don't need to explicitly depend on `testcontainers` as it's re-exported dependency
> of `testcontainers-modules` with aligned version between these crates.
> For example:
>
>```rust
>use testcontainers_modules::testcontainers::RunnableImage;
>```

You can also see [examples](https://github.com/testcontainers/testcontainers-rs-modules-community/tree/main/examples)
for more details.

## 2. How to override module defaults

Sometimes it's necessary to override default settings of the module (e.g `tag`, `name`, environment variables etc.)
In order to do that, just use extension trait [ImageExt](https://docs.rs/testcontainers/latest/testcontainers/core/trait.ImageExt.html)
that returns customized [RunnableImage](https://docs.rs/testcontainers/latest/testcontainers/core/struct.RunnableImage.html):

```rust
use testcontainers_modules::{
    redis::Redis,
    testcontainers::{RunnableImage, ImageExt},
};


/// Create a Redis module with `6.2-alpine` tag and custom password
fn create_redis() -> RunnableImage<Redis> {
    Redis::default()
        .with_tag("6.2-alpine")
        .with_env_var(("REDIS_PASSWORD", "my_secret_password"))
}
```