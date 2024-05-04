# Wait Strategies

There are scenarios where your tests need the external services they rely on to reach a specific state that is particularly useful for testing. This is generally approximated as 'Can we talk to this container over the network?' or 'Let's wait until the container is running an reaches certain state'.

_Testcontainers for Rust_ comes with the concept of `wait strategy`, which allows your tests to actually wait for the most useful conditions to be met, before continuing with their execution.

## Startup timeout and Poll interval

When defining a wait strategy, it should define a way to set the startup timeout to avoid waiting infinitely.
