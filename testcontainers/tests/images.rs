extern crate bitcoin_rpc_client;
extern crate pretty_env_logger;
extern crate spectral;
extern crate testcontainers;
extern crate web3;

use spectral::prelude::*;
use testcontainers::*;

use bitcoin_rpc_client::BitcoinRpcApi;
use web3::futures::Future;
use web3::transports::Http;
use web3::Web3;

#[test]
fn coblox_bitcoincore_getnewaddress() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::coblox_bitcoincore::BitcoinCore::default());

    let client = {
        let host_port = node.get_host_port(18443).unwrap();

        let url = format!("http://localhost:{}", host_port);

        let auth = node.image().auth();

        bitcoin_rpc_client::BitcoinCoreClient::new(url.as_str(), auth.username(), auth.password())
    };

    assert_that(&client.get_new_address()).is_ok().is_ok();
}

#[test]
fn parity_parity_listaccounts() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::parity_parity::ParityEthereum::default());

    let (_event_loop, web3) = {
        let host_port = node.get_host_port(8545).unwrap();

        let url = format!("http://localhost:{}", host_port);

        let (_event_loop, transport) = Http::new(&url).unwrap();
        let web3 = Web3::new(transport);

        (_event_loop, web3)
    };

    let accounts = web3.eth().accounts().wait();

    assert_that(&accounts).is_ok();
}

#[test]
fn trufflesuite_ganachecli_listaccounts() {
    let _ = pretty_env_logger::try_init();
    let docker = clients::Cli::default();
    let node = docker.run(images::trufflesuite_ganachecli::GanacheCli::default());

    let (_event_loop, web3) = {
        let host_port = node.get_host_port(8545).unwrap();

        let url = format!("http://localhost:{}", host_port);

        let (_event_loop, transport) = Http::new(&url).unwrap();
        let web3 = Web3::new(transport);

        (_event_loop, web3)
    };

    let accounts = web3.eth().accounts().wait();

    assert_that(&accounts).is_ok();
}
