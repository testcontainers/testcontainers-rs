extern crate testcontainers;
extern crate trufflesuite_ganachecli;
extern crate web3;

use std::ops::Deref;
use testcontainers::Container;
use testcontainers::{clients::DockerCli, Docker};
use trufflesuite_ganachecli::*;
use web3::futures::Future;
use web3::{
    transports::{EventLoopHandle, Http},
    Web3,
};

fn main() {
    let docker = DockerCli::new();

    // This blocks until the container is ready to accept requests
    let node = docker.run(GanacheCli::default());
    let client = Web3Client::new(&node);

    let accounts = client.eth().accounts().wait().unwrap();

    println!("Available accounts: {:#?}", accounts);

    // The container will removed as soon as the `node` variable goes out of scope.
}

/// We create a custom Web3 client because the eventloop must not be dropped after creation.
/// Our Defer implementation allows us to use this as it was an instance of Web3<Http>
pub struct Web3Client {
    _event_loop: EventLoopHandle,
    web3: Web3<Http>,
}

impl Web3Client {
    pub fn new<D: Docker>(container: &Container<D, GanacheCli>) -> Self {
        let host_port = container.get_host_port(8545).unwrap();

        let url = format!("http://localhost:{}", host_port);

        let (_event_loop, transport) = Http::new(&url).unwrap();
        let web3 = Web3::new(transport);

        Web3Client { _event_loop, web3 }
    }
}

impl Deref for Web3Client {
    type Target = Web3<Http>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.web3
    }
}
