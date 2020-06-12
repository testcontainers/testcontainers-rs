use crate::core::Port;
use crate::core::{Container, Docker, Image, WaitForMessage};
use hex::encode;
use hmac::{Hmac, Mac, NewMac};
use rand::{thread_rng, Rng};
use sha2::Sha256;
use std::{collections::HashMap, env::var, thread::sleep, time::Duration};

#[derive(Debug)]
pub struct BitcoinCore {
    tag: String,
    arguments: BitcoinCoreImageArgs,
    ports: Option<Vec<Port>>,
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
        }
    }
}

impl IntoIterator for BitcoinCoreImageArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        let mut args = Vec::new();

        args.push(format!("-rpcauth={}", self.rpc_auth.encode()));

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

        args.push("-debug".into()); // Needed for message "Flushed wallet.dat"

        args.into_iter()
    }
}

impl Image for BitcoinCore {
    type Args = BitcoinCoreImageArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = void::Void;

    fn descriptor(&self) -> String {
        format!("coblox/bitcoin-core:{}", self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Flushed wallet.dat")
            .unwrap();

        let additional_sleep_period =
            var("BITCOIND_ADDITIONAL_SLEEP_PERIOD").map(|value| value.parse());

        if let Ok(Ok(sleep_period)) = additional_sleep_period {
            let sleep_period = Duration::from_millis(sleep_period);

            log::trace!(
                "Waiting for an additional {:?} for container {}.",
                sleep_period,
                container.id()
            );

            sleep(sleep_period)
        }
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

    fn ports(&self) -> Option<Vec<Port>> {
        self.ports.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        BitcoinCore { arguments, ..self }
    }
}

impl Default for BitcoinCore {
    fn default() -> Self {
        BitcoinCore {
            tag: "0.17.0".into(),
            arguments: BitcoinCoreImageArgs::default(),
            ports: None,
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

    pub fn with_mapped_port<P: Into<Port>>(mut self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());
        self.ports = Some(ports);
        self
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
