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
//! First, you need to define the [`Image`] that you want to run, and then simply call the `start` method on it from either the [`AsyncRunner`] or [`SyncRunner`] trait.
//! This will return you [`ContainerAsync`] or [`Container`] respectively.
//! Containers implement `Drop`. As soon as they go out of scope, the underlying docker container is removed.
//!
//! See examples in the corresponding runner ([`AsyncRunner`] and [`SyncRunner`])
//!
//! # Ecosystem
//!
//! `testcontainers` is the core crate that provides an API for working with containers in a test environment.
//! The only image that is provided by the core crate is the [`GenericImage`], which is a simple wrapper around any docker image.
//!
//! However, it does not provide ready-to-use modules, you can implement your [`Image`]s using the library directly or use community supported [`testcontainers-modules`].
//!
//! # Usage in production code
//!
//! Although nothing inherently prevents testcontainers from being used in production code, the library itself was not designed with that in mind.
//! For example, many methods will panic if something goes wrong but because the usage is intended to be within tests, this is deemed acceptable.
//!
//! [tc_website]: https://testcontainers.org
//! [`Docker`]: https://docker.com
//! [`AsyncRunner`]: runners::AsyncRunner
//! [`SyncRunner`]: runners::SyncRunner
//! [`testcontainers-modules`]: https://crates.io/crates/testcontainers-modules

pub mod core;
pub use crate::core::{containers::*, Image, ImageArgs, RunnableImage};

#[cfg(feature = "watchdog")]
#[cfg_attr(docsrs, doc(cfg(feature = "watchdog")))]
pub(crate) mod watchdog;

/// All available Docker images.
mod images;
pub use images::generic::GenericImage;

pub mod runners;
