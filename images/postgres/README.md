# Postgres

This crate provides `postgresql` version 11 as an `Image` for `testcontainers`.

## Authentication

The `postgres` docker image enables `trust` authentication, meaning you can connect to it without a password on localhost. For this reason, the `Image` currently does not expose any configuration regarding user or password. Please check the [integration-test](./../../testcontainers/tests/images.rs) to see how you can connect to it from Rust.
