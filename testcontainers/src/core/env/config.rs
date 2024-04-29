use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::Deserialize;
use serde_with::serde_as;
use url::Url;

use crate::core::env::GetEnvValue;

const TESTCONTAINERS_PROPERTIES: &str = ".testcontainers.properties";

/// The default `DOCKER_HOST` address that we will try to connect to.
#[cfg(unix)]
pub const DEFAULT_DOCKER_HOST: &str = "unix:///var/run/docker.sock";

/// The default `DOCKER_HOST` address that a windows client will try to connect to.
#[cfg(windows)]
pub const DEFAULT_DOCKER_HOST: &str = "npipe:////./pipe/docker_engine";

#[derive(Debug, Default)]
pub(crate) struct Config {
    tc_host: Option<Url>,
    host: Option<Url>,
    tls_verify: Option<bool>,
    cert_path: Option<PathBuf>,
    command: Option<Command>,
}

#[cfg(feature = "properties-config")]
#[serde_as]
#[derive(Debug, Default, Deserialize)]
struct TestcontainersProperties {
    #[serde(rename = "tc.host")]
    tc_host: Option<Url>,
    #[serde(rename = "docker.host")]
    host: Option<Url>,
    #[serde_as(as = "Option<serde_with::BoolFromInt>")]
    #[serde(rename = "docker.tls.verify")]
    tls_verify: Option<bool>,
    #[serde(rename = "docker.cert.path")]
    cert_path: Option<PathBuf>,
}

#[cfg(feature = "properties-config")]
impl TestcontainersProperties {
    async fn load() -> Option<Self> {
        let home_dir = dirs::home_dir()?;
        let properties_path = home_dir.join(TESTCONTAINERS_PROPERTIES);

        let content = tokio::fs::read(properties_path).await.ok()?;
        let properties =
            serde_java_properties::from_slice(&content).expect("Failed to parse properties");

        Some(properties)
    }
}

impl Config {
    pub(crate) async fn load<E>() -> Self
    where
        E: GetEnvValue,
    {
        let env_config = Self::load_from_env_config::<E>();

        #[cfg(feature = "properties-config")]
        {
            let properties = TestcontainersProperties::load().await.unwrap_or_default();

            // Environment variables take precedence over properties
            Self {
                tc_host: env_config.tc_host.or(properties.tc_host),
                host: env_config.host.or(properties.host),
                tls_verify: env_config.tls_verify.or(properties.tls_verify),
                cert_path: env_config.cert_path.or(properties.cert_path),
                command: env_config.command,
            }
        }
        #[cfg(not(feature = "properties-config"))]
        env_config
    }

    fn load_from_env_config<E>() -> Self
    where
        E: GetEnvValue,
    {
        let host = E::get_env_value("DOCKER_HOST")
            .as_deref()
            .map(FromStr::from_str)
            .transpose()
            .expect("Invalid DOCKER_HOST");
        let tls_verify = E::get_env_value("DOCKER_TLS_VERIFY").map(|v| v == "1");
        let cert_path = E::get_env_value("DOCKER_CERT_PATH").map(PathBuf::from);
        let command = E::get_env_value("TESTCONTAINERS_COMMAND").and_then(|v| v.parse().ok());

        Config {
            host,
            tc_host: None,
            command,
            tls_verify,
            cert_path,
        }
    }

    /// The Docker host to use. The host is resolved in the following order:
    ///  1. Docker host from the "tc.host" property in the ~/.testcontainers.properties file.
    ///  2. DOCKER_HOST environment variable.
    ///  3. Docker host from the "docker.host" property in the ~/.testcontainers.properties file.
    ///  4. Else, the default Docker socket will be returned.
    pub(crate) fn docker_host(&self) -> Url {
        self.tc_host
            .as_ref()
            .or(self.host.as_ref())
            .cloned()
            .unwrap_or_else(|| Url::from_str(DEFAULT_DOCKER_HOST).unwrap())
    }

    pub(crate) fn tls_verify(&self) -> bool {
        self.tls_verify.unwrap_or_default()
    }

    pub(crate) fn cert_path(&self) -> Option<&Path> {
        self.cert_path.as_deref()
    }

    pub(crate) fn command(&self) -> Command {
        self.command.unwrap_or_default()
    }
}

/// The commands available to the `TESTCONTAINERS_COMMAND` env variable.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Command {
    Keep,
    #[default]
    Remove,
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keep" => Ok(Command::Keep),
            "remove" => Ok(Command::Remove),
            other => {
                panic!("unknown command '{other}' provided via TESTCONTAINERS_COMMAND env variable",)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "properties-config")]
    #[test]
    fn deserialize_java_properties() {
        let tc_host = Url::parse("http://tc-host").unwrap();
        let docker_host = Url::parse("http://docker-host").unwrap();
        let tls_verify = 1;
        let cert_path = "/path/to/cert";

        let file_content = format!(
            r"
            tc.host={tc_host}
            docker.host={docker_host}
            docker.tls.verify={tls_verify}
            docker.cert.path={cert_path}
        "
        );
        let properties: TestcontainersProperties =
            serde_java_properties::from_slice(file_content.as_bytes())
                .expect("Failed to parse properties");

        assert_eq!(properties.tc_host, Some(tc_host));
        assert_eq!(properties.host, Some(docker_host));
        assert_eq!(properties.tls_verify, Some(tls_verify == 1));
        assert_eq!(properties.cert_path, Some(PathBuf::from(cert_path)));
    }
}
