# Waiting for containers to start or be ready

There are scenarios where your tests need the external services they rely on to reach a specific state that is particularly useful for testing. This is generally approximated as 'Can we talk to this container over the network?' or 'Let's wait until the container is running an reaches certain state'.

_Testcontainers for
Rust_ comes with the concept of `wait strategy`, which allows your tests to actually wait for
the most useful conditions to be met, before continuing with their execution.

The strategy is defined by the [`WaitFor`](https://docs.rs/testcontainers/latest/testcontainers/core/enum.WaitFor.html)
enum with the following variants:

* `StdOutMessage` - wait for a specific message to appear on the container's stdout
* `StdErrMessage` - wait for a specific message to appear on the container's stderr
* `Healthcheck` - wait for the container to be healthy
* `Http` - wait for an HTTP(S) response with predefined conditions (see [`HttpWaitStrategy`](https://docs.rs/testcontainers/latest/testcontainers/core/wait/struct.HttpWaitStrategy.html) for more details)
* `Duration` - wait for a specific duration. Usually less preferable and better to combine with other strategies.
* `SuccessfulCommand` - wait for a given command to exit successfully (exit code 0).

[`Image`](https://docs.rs/testcontainers/latest/testcontainers/core/trait.Image.html) implementation
is responsible for returning the appropriate `WaitFor` strategies.
For [`GenericImage`](https://docs.rs/testcontainers/latest/testcontainers/struct.GenericImage.html)
you can use the `with_wait_for` method to specify the wait strategy.

## Startup timeout and Poll interval

Ordinarily Testcontainers will wait for up to 60 seconds for containers to start.
If the default 60s timeout is not sufficient, it can be updated with the
[`ImageExt::with_startup_timeout(duration)`](https://docs.rs/testcontainers/latest/testcontainers/core/trait.ImageExt.html#method.with_startup_timeout) method.
