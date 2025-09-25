use std::collections::BTreeMap;

use crate::core::{client::Client, containers::request::Host, error::TestcontainersError};

/// Checks if we're running on Docker Desktop (macOS or Windows).
/// Docker Desktop provides `host.docker.internal` by default.
fn is_docker_desktop() -> bool {
    let is_desktop = cfg!(any(target_os = "macos", target_os = "windows"));
    if is_desktop {
        log::trace!("Platform: Docker Desktop, host.docker.internal available");
    } else {
        log::trace!("Platform: not Docker Desktop");
    }
    is_desktop
}

/// Checks if a Docker version string is at least the specified major.minor version
fn is_docker_version_at_least(version_str: &str, min_major: u32, min_minor: u32) -> bool {
    // Docker version strings can be like "20.10.17", "24.0.0", etc.
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts[1].parse().unwrap_or(0);

    major > min_major || (major == min_major && minor >= min_minor)
}

/// Checks if the current Docker installation supports the `host-gateway` feature.
/// This is available in Docker 20.10+ on Linux.
async fn supports_host_gateway(client: &Client) -> bool {
    if !cfg!(target_os = "linux") {
        log::trace!("Platform: not Linux, host-gateway unsupported");
        return false;
    }

    log::trace!("Platform: Linux, checking Docker version for host-gateway");

    // Check Docker version to see if it supports host-gateway
    match client.docker_version().await {
        Ok(Some(version_str)) => {
            let supported = is_docker_version_at_least(&version_str, 20, 10);
            if !supported {
                log::warn!("Docker version {} detected on Linux - host-gateway may not be supported (requires 20.10+)", version_str);
            }
            supported
        }
        Ok(None) => {
            // No version info available, assume it's supported but warn
            log::warn!(
                "Docker version not available - assuming host-gateway is supported on Linux"
            );
            true
        }
        Err(err) => {
            // If we can't get version info, assume it's supported but warn
            log::warn!(
                "Failed to detect Docker version ({}), assuming host-gateway is supported on Linux",
                err
            );
            true
        }
    }
}

/// Determines the appropriate native method for exposing host ports based on the Docker environment.
///
/// Returns the hostname that should be used for `host.testcontainers.internal` mapping.
/// - On Docker Desktop (macOS/Windows): uses `host.docker.internal`
/// - On Linux with Docker 20.10+: uses `host-gateway`
/// - Returns `None` if no native support is available
pub async fn detect_native_host_mapping(client: &Client) -> Option<Host> {
    log::trace!("Detecting native host mapping method");

    // Try host-gateway first (modern Linux Docker)
    if supports_host_gateway(client).await {
        log::info!("Host mapping: using host-gateway");
        Some(Host::HostGateway)
    }
    // Try host.docker.internal (Docker Desktop)
    else if is_docker_desktop() {
        log::info!("Host mapping: using host.docker.internal");
        Some(Host::Hostname("host.docker.internal".to_string()))
    }
    // No native support available
    else {
        log::warn!("Host mapping: none available, fallback needed");
        None
    }
}

/// Adds the host.testcontainers.internal mapping to the hosts map if host ports are exposed.
///
/// This function tries native Docker support (host-gateway on Linux, host.docker.internal on Docker Desktop).
/// If native support is not available, returns an error.
pub async fn setup_host_mapping(
    client: &Client,
    hosts: &mut BTreeMap<String, Host>,
    exposed_host_ports: &[u16],
) -> Result<(), TestcontainersError> {
    if exposed_host_ports.is_empty() {
        log::info!("No host ports exposed, skipping host mapping setup");
        return Ok(());
    }

    log::trace!(
        "Setting up host mapping for ports: {:?}",
        exposed_host_ports
    );

    // First try native Docker support
    if let Some(host_mapping) = detect_native_host_mapping(client).await {
        log::trace!("Adding host.testcontainers.internal -> {}", host_mapping);
        hosts.insert("host.testcontainers.internal".to_string(), host_mapping);
        return Ok(());
    }

    // If native mapping fails, return an error
    Err(TestcontainersError::HostPortMappingUnavailable)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn test_version_parsing() {
        // Test supported versions
        assert!(is_docker_version_at_least("20.10.17", 20, 10));
        assert!(is_docker_version_at_least("24.0.0", 20, 10));
        assert!(is_docker_version_at_least("21.0.0", 20, 10));

        // Test unsupported versions
        assert!(!is_docker_version_at_least("19.03.12", 20, 10));
        assert!(!is_docker_version_at_least("20.9.0", 20, 10));
        assert!(!is_docker_version_at_least("invalid", 20, 10));

        // Edge cases for version parsing
        assert!(!is_docker_version_at_least("", 20, 10));
        assert!(!is_docker_version_at_least("20", 20, 10));
        assert!(!is_docker_version_at_least("abc.def", 20, 10));
        assert!(is_docker_version_at_least("20.10", 20, 10));
        assert!(is_docker_version_at_least("20.10.0.extra.stuff", 20, 10));
    }

    #[test]
    fn test_setup_host_mapping_no_ports() {
        // This test can remain synchronous since it doesn't actually call the async methods
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            use crate::core::client::Client;

            let client = Client::lazy_client().await.unwrap();
            let mut hosts = BTreeMap::new();
            let ports = vec![];

            setup_host_mapping(&client, &mut hosts, &ports)
                .await
                .unwrap();

            // Should never add host mapping when no ports are exposed
            assert!(!hosts.contains_key("host.testcontainers.internal"));
        });
    }
}
