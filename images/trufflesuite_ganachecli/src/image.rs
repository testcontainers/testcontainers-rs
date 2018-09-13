use testcontainers::{Container, Docker, Image, WaitForMessage};

pub struct GanacheCli {
    tag: String,
    arguments: GanacheCliArgs,
}

#[derive(Clone)]
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
            args.push(format!("{}", self.mnemonic));
        }

        args.push("-a".to_string());
        args.push(format!("{}", self.number_of_accounts));
        args.push("-i".to_string());
        args.push(format!("{}", self.network_id));

        args.into_iter()
    }
}

impl Image for GanacheCli {
    type Args = GanacheCliArgs;

    fn descriptor(&self) -> String {
        format!("trufflesuite/ganache-cli:{}", self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Listening on localhost:")
            .unwrap();
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        GanacheCli { arguments, ..self }
    }
}
