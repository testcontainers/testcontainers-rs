#![deny(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![cfg_attr(docsrs, deny(rustdoc::broken_intra_doc_links))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]

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
//! If you need to build an image first, then you need to define the [`BuildableImage`] that specifies the build context
//! and the Dockerfile, then call the `build_image` method on it from either the [`AsyncBuilder`] or [`SyncBuilder`] trait.
//! This will yield an [`Image`] you could actually start.
//!
//! If you already have a Docker image you can just define your [`Image`] that you want to run, and then simply call the
//! `start` method on it from either the [`AsyncRunner`] or [`SyncRunner`] trait.
//!
//! This will return you [`ContainerAsync`] or [`Container`] respectively.
//! Containers implement `Drop`. As soon as they go out of scope, the underlying docker container is removed.
//! To disable this behavior, you can set ENV variable `TESTCONTAINERS_COMMAND` to `keep`.
//!
//! See examples in the corresponding runner ([`AsyncRunner`] and [`SyncRunner`])
//!
//! ### Docker host resolution
//!
//! You can change the configuration of the Docker host used by the client in two ways:
//! - environment variables
//! - `~/.testcontainers.properties` file (a Java properties file, enabled by the `properties-config` feature)
//!
//! ##### The host is resolved in the following order:
//!
//! 1. Docker host from the `tc.host` property in the `~/.testcontainers.properties` file.
//! 2. `DOCKER_HOST` environment variable.
//! 3. Docker host from the "docker.host" property in the `~/.testcontainers.properties` file.
//! 4. Read the default Docker socket path, without the unix schema. E.g. `/var/run/docker.sock`.
//! 5. Read the rootless Docker socket path, checking in the following alternative locations:
//!    1. `${XDG_RUNTIME_DIR}/.docker/run/docker.sock`.
//!    2. `${HOME}/.docker/run/docker.sock`.
//!    3. `${HOME}/.docker/desktop/docker.sock`.
//! 6. The default Docker socket including schema will be returned if none of the above are set.
//!
//! ### Docker authentication
//!
//! Sometimes the Docker images you use live in a private Docker registry.
//! For that reason, Testcontainers for Rust gives you the ability to read the Docker configuration and retrieve the authentication for a given registry.
//! Configuration is fetched in the following order:
//!
//! 1. `DOCKER_AUTH_CONFIG` environment variable, unmarshalling the string value from its JSON representation and using it as the Docker config.
//! 2. `DOCKER_CONFIG` environment variable, as an alternative path to the directory containing Docker `config.json` file.
//! 3. else it will load the default Docker config file, which lives in the user's home, e.g. `~/.docker/config.json`.
//!
//! # Ecosystem
//!
//! `testcontainers` is the core crate that provides an API for working with containers in a test environment.
//! The only buildable image and image implementations that are provided by the core crate are the [`GenericBuildableImage`]
//! and [`GenericImage`], respectively.
//!
//! However, it does not provide ready-to-use modules, you can implement your [`Image`]s using the library directly or use community supported [`testcontainers-modules`].
//!
//! # Usage in production code
//!
//! Although nothing inherently prevents testcontainers from being used in production code, the library itself was not designed with that in mind.
//!
//! [tc_website]: https://testcontainers.org
//! [`Docker`]: https://docker.com
//! [`AsyncBuilder`]: runners::AsyncBuilder
//! [`SyncBuilder`]: runners::SyncBuilder
//! [`AsyncRunner`]: runners::AsyncRunner
//! [`SyncRunner`]: runners::SyncRunner
//! [`testcontainers-modules`]: https://crates.io/crates/testcontainers-modules

pub mod core;
#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub use crate::core::Container;
#[cfg(feature = "reusable-containers")]
pub use crate::core::ReuseDirective;
pub use crate::core::{
    copy::{CopyDataSource, CopyToContainer, CopyToContainerError},
    error::TestcontainersError,
    BuildableImage, ContainerAsync, ContainerRequest, Healthcheck, Image, ImageExt,
};

#[cfg(feature = "watchdog")]
#[cfg_attr(docsrs, doc(cfg(feature = "watchdog")))]
pub(crate) mod watchdog;

mod buildables;
pub use buildables::generic::GenericBuildableImage;

/// All available Docker images.
mod images;
pub use images::generic::GenericImage;

pub mod runners;

/// Re-export of the `bollard` crate to allow direct interaction with the Docker API.
/// This also solves potential version conflicts between `testcontainers` and `bollard` deps.
pub use bollard;
