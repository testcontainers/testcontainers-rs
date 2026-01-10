use std::{str::FromStr, time::Duration};

use bollard::{Docker, API_DEFAULT_VERSION};
use http::header::{HeaderValue, USER_AGENT};
use url::Url;

use crate::core::env;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const USER_AGENT_VALUE: &str = concat!("tc-rust/", env!("CARGO_PKG_VERSION"));

pub(super) fn init(config: &env::Config) -> Result<Docker, bollard::errors::Error> {
    let host = &config.docker_host();
    let host_url = Url::from_str(host)?;

    let docker = match host_url.scheme() {
        #[cfg(any(feature = "ring", feature = "aws-lc-rs"))]
        "https" => connect_with_ssl(config)?,
        "http" | "tcp" => {
            #[cfg(any(feature = "ring", feature = "aws-lc-rs"))]
            if config.tls_verify() {
                return Ok(connect_with_ssl(config)?.with_request_modifier(add_user_agent));
            }
            Docker::connect_with_http(host, DEFAULT_TIMEOUT.as_secs(), API_DEFAULT_VERSION)?
        }
        #[cfg(unix)]
        "unix" => Docker::connect_with_unix(host, DEFAULT_TIMEOUT.as_secs(), API_DEFAULT_VERSION)?,
        #[cfg(windows)]
        "npipe" => {
            Docker::connect_with_named_pipe(host, DEFAULT_TIMEOUT.as_secs(), API_DEFAULT_VERSION)?
        }
        _ => {
            return Err(bollard::errors::Error::UnsupportedURISchemeError {
                uri: host.to_string(),
            })
        }
    };

    Ok(docker.with_request_modifier(add_user_agent))
}

fn add_user_agent(mut req: bollard::BollardRequest) -> bollard::BollardRequest {
    req.headers_mut()
        .insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    req
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_value_has_correct_format() {
        // Verify the User-Agent follows the tc-rust/<version> format
        assert!(
            USER_AGENT_VALUE.starts_with("tc-rust/"),
            "User-Agent should start with 'tc-rust/', got: {}",
            USER_AGENT_VALUE
        );

        // Verify version part is not empty
        let version = USER_AGENT_VALUE.strip_prefix("tc-rust/").unwrap();
        assert!(!version.is_empty(), "Version should not be empty");

        // Verify version matches Cargo.toml version
        assert_eq!(
            version,
            env!("CARGO_PKG_VERSION"),
            "Version should match CARGO_PKG_VERSION"
        );
    }

    #[test]
    fn user_agent_value_matches_expected() {
        // The version from Cargo.toml
        let expected = concat!("tc-rust/", env!("CARGO_PKG_VERSION"));
        assert_eq!(USER_AGENT_VALUE, expected);
    }
}
