#![deny(missing_debug_implementations)]

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

extern crate tc_cli_client;
extern crate tc_core;

extern crate tc_coblox_bitcoincore;
extern crate tc_dynamodb_local;
extern crate tc_elasticmq;
extern crate tc_parity_parity;
extern crate tc_redis;
extern crate tc_trufflesuite_ganachecli;

/// All available Docker clients.
pub mod clients {
    pub use tc_cli_client::Cli;
}

/// All available Docker images.
pub mod images {
    pub mod coblox_bitcoincore {
        pub use tc_coblox_bitcoincore::{BitcoinCore, BitcoinCoreImageArgs, Network, RpcAuth};
    }

    pub mod parity_parity {
        pub use tc_parity_parity::{ParityEthereum, ParityEthereumArgs};
    }

    pub mod trufflesuite_ganachecli {
        pub use tc_trufflesuite_ganachecli::{GanacheCli, GanacheCliArgs};
    }

    pub mod dynamodb_local {
        pub use tc_dynamodb_local::{DynamoDb, DynamoDbArgs};
    }

    pub mod redis {
        pub use tc_redis::{Redis, RedisArgs};
    }

    pub mod elasticmq {
        pub use tc_elasticmq::{ElasticMQ, ElasticMQArgs};
    }

}

pub use tc_core::*;
