use crate::{core::WaitFor, Image};

#[derive(Debug)]
pub struct GanacheCli {
    tag: String,
    arguments: GanacheCliArgs,
}

#[derive(Debug, Clone)]
pub struct GanacheCliArgs {
    pub network_id: u32,
    pub number_of_accounts: u32,
    pub mnemonic: String,
}

impl Default for GanacheCli {
    fn default() -> Self {
        GanacheCli {
            tag: "v6.1.3".into(),
            arguments: GanacheCliArgs::default(),
        }
    }
}

impl Default for GanacheCliArgs {
    fn default() -> Self {
        GanacheCliArgs {
            network_id: 42,
            number_of_accounts: 7,
            mnemonic: "supersecure".to_string(),
        }
    }
}

impl IntoIterator for GanacheCliArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        let mut args = Vec::new();

        if !self.mnemonic.is_empty() {
            args.push("-m".to_string());
            args.push(self.mnemonic.to_string());
        }

        args.push("-a".to_string());
        args.push(self.number_of_accounts.to_string());
        args.push("-i".to_string());
        args.push(self.network_id.to_string());

        args.into_iter()
    }
}

impl Image for GanacheCli {
    type Args = GanacheCliArgs;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("trufflesuite/ganache-cli:{}", self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Listening on localhost:")]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        GanacheCli { arguments, ..self }
    }
}
