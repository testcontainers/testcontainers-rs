#![deny(missing_debug_implementations)]
#![deprecated(
    since = "0.2.1",
    note = "Testcontainers is no longer using microcrates, please upgrade to testcontainers version 0.8"
)]

#[macro_use]
extern crate log;
extern crate tc_core;

mod image;
pub use image::*;
