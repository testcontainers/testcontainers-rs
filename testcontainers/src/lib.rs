extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate maplit;

mod api;
mod wait_for_message;

pub mod clients;

pub use api::*;
pub use wait_for_message::{WaitError, WaitForMessage};

pub mod prelude {

    pub use Docker;
    pub use Image;

    pub use Container;
    pub use Logs;
    pub use Ports;

    pub use WaitError;
    pub use WaitForMessage;

    pub use clients;
}
