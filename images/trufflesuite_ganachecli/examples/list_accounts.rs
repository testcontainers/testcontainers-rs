extern crate testcontainers;
extern crate trufflesuite_ganachecli;

use testcontainers::{clients::DockerCli, Docker};
use trufflesuite_ganachecli::*;

fn main() {
    let docker = DockerCli::new();

    // This blocks until the container is ready to accept requests
    let node = docker.run(GanacheCli::default());
    let client = node.connect::<Web3Client>();

    let accounts = client.eth().accounts().wait().unwrap();

    println!("Available accounts: {:#?}", accounts);

    // The container will removed as soon as the `node` variable goes out of scope.
}
