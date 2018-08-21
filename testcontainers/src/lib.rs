extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

mod api;
mod docker_cli;
mod wait_for_message;

pub use api::*;
pub use wait_for_message::{WaitError, WaitForMessage};

pub mod clients {
    pub use docker_cli::DockerCli;
}
