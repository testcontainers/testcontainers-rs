use crate::core::env;
use bollard::{Docker, API_DEFAULT_VERSION};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2 * 60);

pub(super) fn init(config: &env::Config) -> Docker {
    let host = config.docker_host();

    match host.scheme() {
        "https" => connect_with_ssl(config),
        "http" | "tcp" => {
            if config.tls_verify() {
                connect_with_ssl(config)
            } else {
                Docker::connect_with_http(
                    host.as_str(),
                    DEFAULT_TIMEOUT.as_secs(),
                    API_DEFAULT_VERSION,
                )
            }
        }
        #[cfg(unix)]
        "unix" => Docker::connect_with_unix(
            host.as_str(),
            DEFAULT_TIMEOUT.as_secs(),
            API_DEFAULT_VERSION,
        ),
        #[cfg(windows)]
        "npipe" => Docker::connect_with_named_pipe(
            host.as_str(),
            DEFAULT_TIMEOUT.as_secs(),
            API_DEFAULT_VERSION,
        ),
        scheme => {
            panic!("Unsupported scheme: {scheme}");
        }
    }
    .expect("Failed to connect to Docker")
}

fn connect_with_ssl(config: &env::Config) -> Result<Docker, bollard::errors::Error> {
    let cert_path = config.cert_path().expect("cert path not found");

    Docker::connect_with_ssl(
        config.docker_host().as_str(),
        &cert_path.join("key.pem"),
        &cert_path.join("cert.pem"),
        &cert_path.join("ca.pem"),
        DEFAULT_TIMEOUT.as_secs(),
        API_DEFAULT_VERSION,
    )
}
