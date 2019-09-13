#![deny(missing_debug_implementations)]
#![deprecated(
    since = "0.4.1",
    note = "Testcontainers is no longer using microcrates, please upgrade to testcontainers version 0.8"
)]

extern crate tc_core;

mod image;

pub use image::{GanacheCli, GanacheCliArgs};
