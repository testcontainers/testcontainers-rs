#![deny(missing_debug_implementations)]
#![deprecated(
    since = "0.2.1",
    note = "Testcontainers is no longer using microcrates, please upgrade to testcontainers version 0.8"
)]

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate tc_core;

mod cli;

pub use self::cli::Cli;
