#![deny(missing_debug_implementations)]
#![deprecated(
    since = "0.5.1",
    note = "Testcontainers is no longer using microcrates, please upgrade to testcontainers version 0.8"
)]

extern crate tc_core;
#[macro_use]
extern crate log;
extern crate hex;
extern crate hmac;
extern crate rand;
extern crate sha2;

mod image;

pub use image::{BitcoinCore, BitcoinCoreImageArgs, Network, RpcAuth};
