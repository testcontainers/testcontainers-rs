use crate::core::{Image, WaitFor};
use hex::encode;
use hmac::{Hmac, Mac, NewMac};
use rand::{thread_rng, Rng};
use sha2::Sha256;
use std::{collections::HashMap, fmt};

const BITCOIND_STARTUP_MESSAGE: &str = "bitcoind startup sequence completed.";

#[derive(Debug)]
pub struct BitcoinCore {
    tag: String,
    arguments: BitcoinCoreImageArgs,
}

impl BitcoinCore {
    pub fn auth(&self) -> &RpcAuth {
        &self.arguments.rpc_auth
    }
}

#[derive(Debug, Clone)]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
}

#[derive(Debug, Clone)]
pub enum AddressType {
    Legacy,
    P2shSegwit,
    Bech32,
}

impl fmt::Display for AddressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AddressType::Legacy => "legacy",
            AddressType::P2shSegwit => "p2sh-segwit",
            AddressType::Bech32 => "bech32",
        })
    }
}

#[derive(Clone, Debug)]
pub struct RpcAuth {
    pub username: String,
    pub password: String,
    pub salt: String,
}

impl RpcAuth {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn new(username: String) -> Self {
        let salt = Self::generate_salt();
        let password = Self::generate_password();

        RpcAuth {
            username,
            password,
            salt,
        }
    }

    fn generate_salt() -> String {
        let mut buffer = [0u8; 16];
        thread_rng().fill(&mut buffer[..]);
        encode(buffer)
    }

    fn generate_password() -> String {
        let mut buffer = [0u8; 32];
        thread_rng().fill(&mut buffer[..]);

        encode(buffer)
    }

    fn encode_password(&self) -> String {
        let mut mac = Hmac::<Sha256>::new_varkey(self.salt.as_bytes()).unwrap();
        mac.update(self.password.as_bytes().as_ref());

        let result = mac.finalize().into_bytes();

        encode(result)
    }

    pub fn encode(&self) -> String {
        format!("{}:{}${}", self.username, self.salt, self.encode_password())
    }
}

#[derive(Debug, Clone)]
pub struct BitcoinCoreImageArgs {
    pub server: bool,
    pub network: Network,
    pub print_to_console: bool,
    pub tx_index: bool,
    pub rpc_bind: String,
    pub rpc_allowip: String,
    pub rpc_auth: RpcAuth,
    pub accept_non_std_txn: Option<bool>,
    pub rest: bool,
    pub fallback_fee: Option<f64>,
    pub address_type: AddressType,
}

impl Default for BitcoinCoreImageArgs {
    fn default() -> Self {
        BitcoinCoreImageArgs {
            server: true,
            network: Network::Regtest,
            print_to_console: true,
            rpc_auth: RpcAuth::new(String::from("bitcoin")),
            tx_index: true,
            rpc_bind: "0.0.0.0".to_string(), // This allows to bind on all ports
            rpc_allowip: "0.0.0.0/0".to_string(),
            accept_non_std_txn: Some(false),
            rest: true,
            fallback_fee: Some(0.0002),
            address_type: AddressType::Bech32,
        }
    }
}

impl IntoIterator for BitcoinCoreImageArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        let mut args = vec![
            format!("-rpcauth={}", self.rpc_auth.encode()),
            // Will print a message when bitcoind is fully started
            format!("-startupnotify='echo \'{}\''", BITCOIND_STARTUP_MESSAGE),
            format!("-addresstype={}", self.address_type),
        ];

        if self.server {
            args.push("-server".to_string())
        }

        match self.network {
            Network::Testnet => args.push("-testnet".to_string()),
            Network::Regtest => args.push("-regtest".to_string()),
            Network::Mainnet => {}
        }

        if self.tx_index {
            args.push("-txindex=1".to_string())
        }

        if !self.rpc_allowip.is_empty() {
            args.push(format!("-rpcallowip={}", self.rpc_allowip));
        }

        if !self.rpc_bind.is_empty() {
            args.push(format!("-rpcbind={}", self.rpc_bind));
        }

        if self.print_to_console {
            args.push("-printtoconsole".to_string())
        }

        if let Some(accept_non_std_txn) = self.accept_non_std_txn {
            if accept_non_std_txn {
                args.push("-acceptnonstdtxn=1".to_string());
            } else {
                args.push("-acceptnonstdtxn=0".to_string());
            }
        }

        if self.rest {
            args.push("-rest".to_string())
        }

        if let Some(fallback_fee) = self.fallback_fee {
            args.push(format!("-fallbackfee={}", fallback_fee));
        }

        args.into_iter()
    }
}

impl Image for BitcoinCore {
    type Args = BitcoinCoreImageArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("coblox/bitcoin-core:{}", self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout(BITCOIND_STARTUP_MESSAGE),
            WaitFor::millis_in_env_var("BITCOIND_ADDITIONAL_SLEEP_PERIOD"),
        ]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        BitcoinCore { arguments, ..self }
    }
}

impl Default for BitcoinCore {
    fn default() -> Self {
        BitcoinCore {
            tag: "0.21.0".into(),
            arguments: BitcoinCoreImageArgs::default(),
        }
    }
}

impl BitcoinCore {
    pub fn with_tag(self, tag_str: &str) -> Self {
        BitcoinCore {
            tag: tag_str.to_string(),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_rpc_auth_correctly() {
        let auth = RpcAuth {
            username: "bitcoin".to_string(),
            password: "54pLR_f7-G6is32LP-7nbhzZSbJs_2zSATtZV_r05yg=".to_string(),
            salt: "cb77f0957de88ff388cf817ddbc7273".to_string(),
        };

        let rpc_auth = auth.encode();

        assert_eq!(rpc_auth, "bitcoin:cb77f0957de88ff388cf817ddbc7273$9eaa166ace0d94a29c6eceb831a42458e93faeb79f895a7ee4ce03f4343f8f55".to_string())
    }
}
