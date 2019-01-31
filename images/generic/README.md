# Postgres

This crate provides a generic `Image` for `testcontainers`.

The `GenericImage` allows to use of whatever Docker Image for which a specific implementation
is not provided by `testcontainers`. 

E.g.:
```rust
// Creates a generic image with descriptor "postgres:9.6-alpine"
let generic_postgres = images::generic::GenericImage::new("postgres:9.6-alpine")
    // Instructs testcontainers to wait until the LogMessage
    // "database system is ready to accept connections" is printed in the
    // container system output
    .with_wait_for(images::generic::WaitFor::LogMessage(
        "database system is ready to accept connections".to_owned(),
    ));

// Runs the it
let node = docker.run(generic_postgres);
```