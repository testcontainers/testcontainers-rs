#![deny(missing_debug_implementations)]
#![deprecated(
    since = "0.3.1",
    note = "Testcontainers is no longer using microcrates, please upgrade to testcontainers version 0.8"
)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate debug_stub_derive;

mod container;
mod docker;
mod image;
mod wait_for_message;

pub use self::container::Container;
pub use self::docker::{Docker, Logs, Ports};
pub use self::image::Image;
pub use self::wait_for_message::{WaitError, WaitForMessage};
