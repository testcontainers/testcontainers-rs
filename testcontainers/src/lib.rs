#![deny(missing_debug_implementations)]
#![warn(rust_2018_idioms)]

//! A library for integration testing against docker containers from within Rust.
//!
//! This crate is the official Rust language fork of [`Testcontainers`][tc_website].
//!
//! Tests should be self-contained and isolated. While this is usually easy for unit-tests, integration-tests typically require a more complex environment.
//! The testcontainers ecosystem facilitates self-contained and isolated integration tests. It allows to easily spin up Docker containers from within your tests and removes them afterwards.
//!
//! A very typical usecase for testcontainers are integration-tests of persistence layers. These require an actual database to be present. Using testcontainers, your tests can spin up database containers themselves, without the need for any other setup.
//!
//! # Main benefits
//!
//! - Run integration tests in parallel (because each test sets up its own environment)
//! - Run integration tests the same way you run unit tests (`cargo test` and you are fine)
//!
//! # Usage
//!
//! Unsurprisingly, working with testcontainers is very similar to working with Docker itself.
//!
//! First you choose a [`Client`]. Given a client instance, you can [`run`][docker_run] [`Images`]. This gives you back a [`Container`]. Containers implement `Drop`. As soon as they go out of scope, the underlying docker container is removed.
//!
//! # Usage in production code
//!
//! Although nothing inherently prevents testcontainers from being used in production code, the library itself was not designed with that in mind. For example, many methods will panic if something goes wrong but because the usage is intended to be within tests, this is deemed acceptable.
//!
//! # Ecosystem
//!
//! The testcontainers ecosystem is split into multiple crates, however, the `testcontainers` crate itself is a meta-crate, re-exporting the others. Usually, depending on `testcontainers` should be sufficient for most users needs.
//!
//! [tc_website]: https://testcontainers.org
//! [`Docker`]: https://docker.com
//! [docker_run]: trait.Docker.html#tymethod.run
//! [`Client`]: trait.Docker.html#implementors
//! [`Images`]: trait.Image.html#implementors
//! [`Container`]: struct.Container.html
pub use crate::core::{Container, Image, ImageArgs, RunnableImage};

#[cfg(feature = "experimental")]
pub use crate::core::ContainerAsync;

#[cfg(feature = "watchdog")]
pub(crate) mod watchdog;

/// All available Docker clients.
pub mod clients;
pub mod core;
/// All available Docker images.
pub mod images;
