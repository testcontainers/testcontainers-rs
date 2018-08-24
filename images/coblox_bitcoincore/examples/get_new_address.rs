extern crate bitcoin_rpc;
extern crate coblox_bitcoincore;
extern crate testcontainers;

use bitcoin_rpc::BitcoinCoreClient;
use bitcoin_rpc::BitcoinRpcApi;
use coblox_bitcoincore::BitcoinCore;
use testcontainers::clients::DockerCli;
use testcontainers::Docker;

fn main() {
    let docker = DockerCli::new();
    let node = docker.run(BitcoinCore::default());

    let client = node.connect(|container| {
        let host_port = container.get_host_port(18443).unwrap();

        let url = format!("http://localhost:{}", host_port);

        let auth = container.image().auth();

        BitcoinCoreClient::new(url.as_str(), auth.username(), auth.password())
    });

    let address = client.get_new_address().unwrap().unwrap();

    println!("Generated address: {:?}", address);
}
