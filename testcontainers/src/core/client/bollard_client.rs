use std::{pin::Pin, str::FromStr, sync::Arc, time::Duration};

use bollard::{errors::Error, BollardRequest, Docker, API_DEFAULT_VERSION};
use futures::Future;
#[cfg(windows)]
use hex;
use http::header::{HeaderValue, USER_AGENT};
use hyper::Response;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use url::Url;

use crate::core::env;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const USER_AGENT_VALUE: &str = concat!("tc-rust/", env!("CARGO_PKG_VERSION"));

type TransportFuture =
    Pin<Box<dyn Future<Output = Result<Response<hyper::body::Incoming>, Error>> + Send>>;

pub(super) fn init(config: &env::Config) -> Result<Docker, Error> {
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
            connect_with_http(host)
        }
        #[cfg(unix)]
        "unix" => connect_with_unix(host),
        #[cfg(windows)]
        "npipe" => connect_with_named_pipe(host),
        _ => Err(Error::UnsupportedURISchemeError {
            uri: host.to_string(),
        }),
    }
}

fn connect_with_http(host: &str) -> Result<Docker, Error> {
    let connector = hyper_util::client::legacy::connect::HttpConnector::new();
    let client = Arc::new(
        Client::builder(TokioExecutor::new())
            .pool_max_idle_per_host(0)
            .build(connector),
    );

    let transport = move |req: BollardRequest| -> TransportFuture {
        let client = Arc::clone(&client);
        Box::pin(async move {
            let (mut parts, body) = req.into_parts();
            parts
                .headers
                .insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
            let req = hyper::Request::from_parts(parts, body);
            client.request(req).await.map_err(Error::from)
        })
    };

    Docker::connect_with_custom_transport(
        transport,
        Some(host),
        DEFAULT_TIMEOUT.as_secs(),
        API_DEFAULT_VERSION,
    )
}

#[cfg(unix)]
fn connect_with_unix(path: &str) -> Result<Docker, Error> {
    let socket_path = path.strip_prefix("unix://").unwrap_or(path);

    if !std::path::Path::new(socket_path).exists() {
        return Err(Error::DockerResponseServerError {
            status_code: 404,
            message: format!("Unix socket not found: {socket_path}"),
        });
    }

    let connector = hyperlocal::UnixConnector;
    let client = Arc::new(
        Client::builder(TokioExecutor::new())
            .pool_max_idle_per_host(0)
            .build(connector),
    );

    let socket_path_owned = socket_path.to_owned();
    let transport = move |req: BollardRequest| -> TransportFuture {
        let client = Arc::clone(&client);
        let socket_path = socket_path_owned.clone();
        Box::pin(async move {
            let (mut parts, body) = req.into_parts();
            parts
                .headers
                .insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));

            // Transform URI to hyperlocal format
            // Bollard sends URIs like "http://localhost/v1.47/containers/json"
            // We need to extract the path and convert to hyperlocal format
            let request_path = parts
                .uri
                .path_and_query()
                .map_or("/", |pq| pq.as_str())
                .to_owned();

            let hyperlocal_uri: hyper::Uri =
                hyperlocal::Uri::new(&socket_path, &request_path).into();
            parts.uri = hyperlocal_uri;

            let req = hyper::Request::from_parts(parts, body);
            client.request(req).await.map_err(Error::from)
        })
    };

    // Use http://localhost so bollard creates valid HTTP URIs that we transform
    Docker::connect_with_custom_transport(
        transport,
        Some("http://localhost"),
        DEFAULT_TIMEOUT.as_secs(),
        API_DEFAULT_VERSION,
    )
}

#[cfg(windows)]
fn connect_with_named_pipe(path: &str) -> Result<Docker, Error> {
    let pipe_path = path.strip_prefix("npipe://").unwrap_or(path);

    let connector = hyper_named_pipe::PipeConnector;
    let client = Arc::new(
        Client::builder(TokioExecutor::new())
            .pool_max_idle_per_host(0)
            .build(connector),
    );

    let pipe_path_owned = pipe_path.to_owned();
    let transport = move |req: BollardRequest| -> TransportFuture {
        let client = Arc::clone(&client);
        let pipe_path = pipe_path_owned.clone();
        Box::pin(async move {
            let (mut parts, body) = req.into_parts();
            parts
                .headers
                .insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));

            // Transform URI to named pipe format
            // The pipe path needs to be hex-encoded for hyper-named-pipe
            let request_path = parts
                .uri
                .path_and_query()
                .map_or("/", |pq| pq.as_str())
                .to_owned();

            let hex_path = hex::encode(&pipe_path);
            let pipe_uri: hyper::Uri = format!("net.pipe://localhost/{}{}", hex_path, request_path)
                .parse()
                .expect("valid URI");
            parts.uri = pipe_uri;

            let req = hyper::Request::from_parts(parts, body);
            client.request(req).await.map_err(Error::from)
        })
    };

    // Use http://localhost so bollard creates valid HTTP URIs that we transform
    Docker::connect_with_custom_transport(
        transport,
        Some("http://localhost"),
        DEFAULT_TIMEOUT.as_secs(),
        API_DEFAULT_VERSION,
    )
}

#[cfg(any(feature = "ring", feature = "aws-lc-rs"))]
fn connect_with_ssl(config: &env::Config) -> Result<Docker, Error> {
    // For SSL, we fall back to bollard's built-in SSL support since it handles
    // certificate loading and rustls configuration. The User-Agent header won't
    // be added for SSL connections until bollard provides a simpler API.
    // This is a known limitation tracked in:
    // https://github.com/testcontainers/testcontainers-rs/issues/576
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
