extern crate bitcoin_rpc_client;
extern crate tc_coblox_bitcoincore;
extern crate testcontainers;

use bitcoin_rpc_client::BitcoinCoreClient;
use bitcoin_rpc_client::BitcoinRpcApi;
use tc_coblox_bitcoincore::BitcoinCore;
use testcontainers::clients::DockerCli;
use testcontainers::Docker;

fn main() {
    let docker = DockerCli::new();
    let node = docker.run(BitcoinCore::default());

    let client = {
        let host_port = node.get_host_port(18443).unwrap();

        let url = format!("http://localhost:{}", host_port);

        let auth = node.image().auth();

        BitcoinCoreClient::new(url.as_str(), auth.username(), auth.password())
    };

    let address = client.get_new_address().unwrap().unwrap();

    println!("Generated address: {:?}", address);
}
