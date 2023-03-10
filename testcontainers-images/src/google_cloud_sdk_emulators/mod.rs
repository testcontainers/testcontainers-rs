use testcontainers::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "google/cloud-sdk";
const TAG: &str = "362.0.0-emulators";

const HOST: &str = "0.0.0.0";
pub const BIGTABLE_PORT: u16 = 8086;
pub const DATASTORE_PORT: u16 = 8081;
pub const FIRESTORE_PORT: u16 = 8080;
pub const PUBSUB_PORT: u16 = 8085;
pub const SPANNER_PORT: u16 = 9010;

#[derive(Debug, Clone)]
pub struct CloudSdkArgs {
    pub host: String,
    pub port: u16,
    pub emulator: Emulator,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Emulator {
    Bigtable,
    Datastore { project: String },
    Firestore,
    PubSub,
    Spanner,
}

impl ImageArgs for CloudSdkArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        let (emulator, project) = match &self.emulator {
            Emulator::Bigtable => ("bigtable", None),
            Emulator::Datastore { project } => ("datastore", Some(project)),
            Emulator::Firestore => ("firestore", None),
            Emulator::PubSub => ("pubsub", None),
            Emulator::Spanner => ("spanner", None),
        };
        let mut args = vec![
            "gcloud".to_owned(),
            "beta".to_owned(),
            "emulators".to_owned(),
            emulator.to_owned(),
            "start".to_owned(),
        ];
        if let Some(project) = project {
            args.push("--project".to_owned());
            args.push(project.to_owned());
        }
        args.push("--host-port".to_owned());
        args.push(format!("{}:{}", self.host, self.port));

        Box::new(args.into_iter())
    }
}

#[derive(Debug)]
pub struct CloudSdk {
    exposed_port: u16,
    ready_condition: WaitFor,
}

impl Image for CloudSdk {
    type Args = CloudSdkArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![self.ready_condition.clone()]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![self.exposed_port]
    }
}

impl CloudSdk {
    fn new(port: u16, emulator: Emulator, ready_condition: WaitFor) -> (Self, CloudSdkArgs) {
        let arguments = CloudSdkArgs {
            host: HOST.to_owned(),
            port,
            emulator,
        };
        let exposed_port = port;
        (
            Self {
                exposed_port,
                ready_condition,
            },
            arguments,
        )
    }

    pub fn bigtable() -> (Self, CloudSdkArgs) {
        Self::new(
            BIGTABLE_PORT,
            Emulator::Bigtable,
            WaitFor::message_on_stderr("[bigtable] Cloud Bigtable emulator running on"),
        )
    }

    pub fn firestore() -> (Self, CloudSdkArgs) {
        Self::new(
            FIRESTORE_PORT,
            Emulator::Firestore,
            WaitFor::message_on_stderr("[firestore] Dev App Server is now running"),
        )
    }

    pub fn datastore(project: impl Into<String>) -> (Self, CloudSdkArgs) {
        let project = project.into();
        Self::new(
            DATASTORE_PORT,
            Emulator::Datastore { project },
            WaitFor::message_on_stderr("[datastore] Dev App Server is now running"),
        )
    }

    pub fn pubsub() -> (Self, CloudSdkArgs) {
        Self::new(
            PUBSUB_PORT,
            Emulator::PubSub,
            WaitFor::message_on_stderr("[pubsub] INFO: Server started, listening on"),
        )
    }

    pub fn spanner() -> (Self, CloudSdkArgs) {
        Self::new(
            SPANNER_PORT, // gRPC port
            Emulator::Spanner,
            WaitFor::message_on_stderr("Cloud Spanner emulator running"),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::google_cloud_sdk_emulators;
    use std::ops::Range;
    use testcontainers::clients;

    const RANDOM_PORTS: Range<u16> = 32768..61000;

    #[test]
    fn bigtable_emulator_expose_port() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(google_cloud_sdk_emulators::CloudSdk::bigtable());
        assert!(RANDOM_PORTS
            .contains(&node.get_host_port_ipv4(google_cloud_sdk_emulators::BIGTABLE_PORT)));
    }

    #[test]
    fn datastore_emulator_expose_port() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(google_cloud_sdk_emulators::CloudSdk::datastore("test"));
        assert!(RANDOM_PORTS
            .contains(&node.get_host_port_ipv4(google_cloud_sdk_emulators::DATASTORE_PORT)));
    }

    #[test]
    fn firestore_emulator_expose_port() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(google_cloud_sdk_emulators::CloudSdk::firestore());
        assert!(RANDOM_PORTS
            .contains(&node.get_host_port_ipv4(google_cloud_sdk_emulators::FIRESTORE_PORT)));
    }

    #[test]
    fn pubsub_emulator_expose_port() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(google_cloud_sdk_emulators::CloudSdk::pubsub());
        assert!(RANDOM_PORTS
            .contains(&node.get_host_port_ipv4(google_cloud_sdk_emulators::PUBSUB_PORT)));
    }

    #[test]
    fn spanner_emulator_expose_port() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(google_cloud_sdk_emulators::CloudSdk::spanner());
        assert!(RANDOM_PORTS
            .contains(&node.get_host_port_ipv4(google_cloud_sdk_emulators::SPANNER_PORT)));
    }
}
