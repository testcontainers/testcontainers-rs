#![deny(missing_debug_implementations)]

extern crate tc_cli_client;
extern crate tc_core;

extern crate tc_coblox_bitcoincore;
extern crate tc_parity_parity;
extern crate tc_trufflesuite_ganachecli;

pub mod clients {
    pub use tc_cli_client::Cli;
}

pub mod images {
    pub mod coblox_bitcoincore {
        pub use tc_coblox_bitcoincore::BitcoinCore;
    }

    pub mod parity_parity {
        pub use tc_parity_parity::{ParityEthereum, ParityEthereumArgs};
    }

    pub mod trufflesuite_ganachecli {
        pub use tc_trufflesuite_ganachecli::{GanacheCli, GanacheCliArgs};
    }
}

pub use tc_core::*;
