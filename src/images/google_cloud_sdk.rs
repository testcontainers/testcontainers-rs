use crate::{core::WaitFor, Image};
use std::collections::HashMap;

const CONTAINER_IDENTIFIER: &str = "google/cloud-sdk";
const DEFAULT_TAG: &str = "353.0.0";

const HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 80;
pub const BIGTABLE_PORT: u16 = 8086;
pub const DATASTORE_PORT: u16 = 8081;
pub const FIRESTORE_PORT: u16 = 8080;
pub const PUBSUB_PORT: u16 = 8085;

const DEFAULT_DATASTORE_PROJECT: &str = "test";

#[derive(Debug, Clone, Default)]
pub struct CloudSdkArgs {
    pub host: String,
    pub port: u16,
    pub project: String,
    pub emulator: Emulator,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Emulator {
    Bigtable,
    Datastore,
    Firestore,
    PubSub,
    Help,
}

impl Default for Emulator {
    fn default() -> Self {
        Emulator::Help
    }
}

impl IntoIterator for CloudSdkArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        let mut args: Vec<String> = ["gcloud", "beta", "emulators"]
            .iter()
            .map(|&s| s.to_owned())
            .collect();
        args.push(
            match self.emulator {
                Emulator::Bigtable => "bigtable",
                Emulator::Datastore => "datastore",
                Emulator::Firestore => "firestore",
                Emulator::PubSub => "pubsub",
                Emulator::Help => "--help",
            }
            .to_owned(),
        );
        args.push("start".to_owned());
        if self.emulator != Emulator::Help {
            if self.emulator == Emulator::Datastore {
                args.push("--project".to_owned());
                args.push(self.project);
            }
            args.push("--host-port".to_owned());
            args.push(format!("{}:{}", self.host, self.port));
        }
        args.into_iter()
    }
}

#[derive(Debug)]
pub struct CloudSdk {
    tag: String,
    arguments: CloudSdkArgs,
    exposed_port: u16,
    ready_condition: WaitFor,
}

impl Default for CloudSdk {
    fn default() -> Self {
        CloudSdk {
            tag: DEFAULT_TAG.to_owned(),
            arguments: CloudSdkArgs::default(),
            exposed_port: DEFAULT_PORT,
            ready_condition: WaitFor::Nothing,
        }
    }
}

impl Image for CloudSdk {
    type Args = CloudSdkArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![self.ready_condition.clone()]
    }

    fn args(&self) -> <Self as Image>::Args {
        self.arguments.clone()
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn with_args(self, arguments: <Self as Image>::Args) -> Self {
        CloudSdk { arguments, ..self }
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![self.exposed_port]
    }
}

impl CloudSdk {
    pub fn new(emulator: Emulator) -> Self {
        let tag = DEFAULT_TAG.to_owned();
        let (exposed_port, ready_condition) = match &emulator {
            Emulator::Bigtable => (
                BIGTABLE_PORT,
                WaitFor::message_on_stderr("[bigtable] Cloud Bigtable emulator running on"),
            ),
            Emulator::Datastore => (
                DATASTORE_PORT,
                WaitFor::message_on_stderr("[datastore] Dev App Server is now running"),
            ),
            Emulator::Firestore => (
                FIRESTORE_PORT,
                WaitFor::message_on_stderr("[firestore] Dev App Server is now running"),
            ),
            Emulator::PubSub => (
                PUBSUB_PORT,
                WaitFor::message_on_stderr("[pubsub] INFO: Server started, listening on"),
            ),
            _ => (DEFAULT_PORT, WaitFor::Nothing),
        };
        let arguments = CloudSdkArgs {
            host: HOST.to_owned(),
            port: exposed_port,
            project: if emulator == Emulator::Datastore {
                DEFAULT_DATASTORE_PROJECT.to_owned()
            } else {
                String::default()
            },
            emulator,
        };
        Self {
            tag,
            arguments,
            exposed_port,
            ready_condition,
        }
    }

    pub fn with_exposed_port(self, port: u16) -> Self {
        Self {
            exposed_port: port,
            arguments: CloudSdkArgs {
                port,
                ..self.arguments
            },
            ..self
        }
    }

    pub fn with_tag(self, tag_str: &str) -> Self {
        CloudSdk {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
