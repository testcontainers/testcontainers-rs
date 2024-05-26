# Waiting for containers to start or be ready

There are scenarios where your tests need the external services they rely on to reach a specific state that is particularly useful for testing. This is generally approximated as 'Can we talk to this container over the network?' or 'Let's wait until the container is running an reaches certain state'.

_Testcontainers for
Rust_ comes with the concept of `wait strategy`, which allows your tests to actually wait for
the most useful conditions to be met, before continuing with their execution.

The strategy is defined by the [`WaitFor`] enum with the following variants:

- `StdOutMessage` - wait for a specific message to appear on the container's stdout
- `StdErrMessage` - wait for a specific message to appear on the container's stderr
- `Healthcheck` - wait for the container to be healthy
- `Duration` - wait for a specific duration. Usually less preferable and better to combine with other strategies.

[`Image`] implementation is responsible for returning the appropriate `WaitFor` strategies.
For [`GenericImage`] you can use the `with_wait_for` method to specify the wait strategy.

## Startup timeout and Poll interval

Ordinarily Testcontainers will wait for up to 60 seconds for containers to start.
If the default 60s timeout is not sufficient, it can be updated with the
[`RunnableImage::with_startup_timeout(duration)`] method.


[`RunnableImage::with_startup_timeout(duration)`]: https://docs.rs/testcontainers/0.17.0/testcontainers/core/struct.RunnableImage.html#method.with_startup_timeout

[`Image`]: https://docs.rs/testcontainers/0.17.0/testcontainers/core/trait.Image.html

[`WaitFor`]: https://docs.rs/testcontainers/0.17.0/testcontainers/core/enum.WaitFor.html

[`GenericImage`]: https://docs.rs/testcontainers/0.17.0/testcontainers/struct.GenericImage.html