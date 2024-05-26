_Testcontainers for Rust_ plays well with the native `cargo test`.

The ideal use case is for integration or end to end tests. It helps you to spin
up and manage the dependencies life cycle via Docker.

## 1. System requirements

Please read the [system requirements](../system_requirements/) page before you start.

## 2. Install _Testcontainers for Rust_

- If your tests are async:
```sh
cargo add testcontainers
```
- If you don't use async and want to use the blocking API:
```sh
cargo add testcontainers --features blocking
```

## 3. Spin up Redis

```rust
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};

#[tokio::test]
async fn test_redis() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let container = GenericImage::new("redis", "7.2.4")
        .with_exposed_port(6379)
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()?
        .await;
}
```

Here we use the `GenericImage` struct to create a Redis container.

* `GenericImage::new` accepts the image name and tag.
* `with_exposed_port` adds a port to be exposed from the container (can be called multiple times).
* `with_wait_for` allows to pass conditions (`WaitFor`) of container rediness. It
  is important to get this set because it helps to know when the container is
  ready to receive any traffic. In this case, we check for the logs we know come
  from Redis, telling us that it is ready to accept requests.
* `start` is a function of the `AsyncRunner` trait that starts the container.
  The same logic is applicable for `SyncRunner` if you are using `blocking` feature.

When you use `with_exposed_port` you have to imagine yourself using `docker run -p
<port>`. When you do so, `dockerd` maps the selected `<port>` from inside the
container to a random one available on your host.

In the previous example, we expose `6379` for `tcp` traffic to the outside. This
allows Redis to be reachable from your code that runs outside the container, but
it also makes parallelization possible. When you run multiple cargo tests in parallel,
each test starts a Redis container, and each of them is exposed on a different random port.

All the containers must be removed at some point, otherwise they will run until
the host is overloaded. In order to provide a clean environment, we rely on `RAII` semantic
of containers (`Drop` trait). Thus, when the container goes out of scope, it is removed by default.
However, you can change this behavior by setting the `TESTCONTAINERS_COMMAND` environment
variable to `keep`.

## 4. Make your code to talk with the container

We will use [redis](https://github.com/redis-rs/redis-rs) as a client in this example.
This code gets the endpoint from the container we just started, and it configures the client.

> This is just an example, you can choose any client you want (e.g [`fred`](https://github.com/aembke/fred.rs))

```rust
use redis::Client;
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};

#[tokio::test]
async fn test_redis() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let container = GenericImage::new("redis", "7.2.4")
        .with_exposed_port(6379)
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .start()?
        .await;
    let host = container.get_host()?;
    let host_port = container.get_host_port_ipv4(REDIS_PORT)?;

    let url = format!("redis://{host}:{host_port}");
    let client = redis::Client::open(url.as_ref())?;
    // do something with the client
}
```

* `get_host` returns the host that this container may be reached on (may not be the local machine).
  In most of the cases it will be `localhost`.
* `get_host_port_ipv4` returns the mapped host port for an internal port of this docker container.
  In this case it returns the port that was exposed by the container.

## 5. Run the test

You can run the test via `cargo test`
