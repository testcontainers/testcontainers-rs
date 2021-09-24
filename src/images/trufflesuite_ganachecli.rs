use crate::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "trufflesuite/ganache-cli";
const TAG: &str = "v6.1.3";

#[derive(Debug, Default)]
pub struct GanacheCli;

#[derive(Debug, Clone)]
pub struct GanacheCliArgs {
    pub network_id: u32,
    pub number_of_accounts: u32,
    pub mnemonic: String,
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

impl ImageArgs for GanacheCliArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        let mut args = Vec::new();

        if !self.mnemonic.is_empty() {
            args.push("-m".to_string());
            args.push(self.mnemonic.to_string());
        }

        args.push("-a".to_string());
        args.push(self.number_of_accounts.to_string());
        args.push("-i".to_string());
        args.push(self.network_id.to_string());

        Box::new(args.into_iter())
    }
}

impl Image for GanacheCli {
    type Args = GanacheCliArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Listening on localhost:")]
    }
}
