use std::{str::FromStr, time::Duration};

use bollard::{Docker, API_DEFAULT_VERSION};
use url::Url;

use crate::core::env;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2 * 60);

pub(super) fn init(config: &env::Config) -> Result<Docker, bollard::errors::Error> {
    let host = &config.docker_host();
    let host_url = Url::from_str(host)?;

    match host_url.scheme() {
        #[cfg(any(feature = "ring", feature = "aws-lc-rs"))]
        "https" => connect_with_ssl(config),
        "http" | "tcp" => {
            #[cfg(any(feature = "ring", feature = "aws-lc-rs"))]
            if config.tls_verify() {
                return connect_with_ssl(config);
            }
            Docker::connect_with_http(host, DEFAULT_TIMEOUT.as_secs(), API_DEFAULT_VERSION)
        }
        #[cfg(unix)]
        "unix" => Docker::connect_with_unix(host, DEFAULT_TIMEOUT.as_secs(), API_DEFAULT_VERSION),
        #[cfg(windows)]
        "npipe" => {
            Docker::connect_with_named_pipe(host, DEFAULT_TIMEOUT.as_secs(), API_DEFAULT_VERSION)
        }
        _ => Err(bollard::errors::Error::UnsupportedURISchemeError {
            uri: host.to_string(),
        }),
    }
}

#[cfg(any(feature = "ring", feature = "aws-lc-rs"))]
fn connect_with_ssl(config: &env::Config) -> Result<Docker, bollard::errors::Error> {
    let cert_path = config.cert_path().expect("cert path not found");

    Docker::connect_with_ssl(
        &config.docker_host(),
        &cert_path.join("key.pem"),
        &cert_path.join("cert.pem"),
        &cert_path.join("ca.pem"),
        DEFAULT_TIMEOUT.as_secs(),
        API_DEFAULT_VERSION,
    )
}
