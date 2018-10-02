#![deny(missing_debug_implementations)]

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate tc_core;

mod cli;

pub use self::cli::Cli;
