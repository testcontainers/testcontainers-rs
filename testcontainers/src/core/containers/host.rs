use std::{convert::TryFrom, future, sync::Arc, time::Duration};

use ferroid::{base32::Base32UlidExt, id::ULID};
use log::{debug, trace};
use russh::{client, Channel, Disconnect};
use tokio::{
    io::{self, copy_bidirectional, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use url::Host as UrlHost;

use super::async_container::ContainerAsync;
use crate::{
    core::{
        async_drop,
        containers::{ContainerRequest, Host},
        error::TestcontainersError,
        image::ImageExt,
        ports::IntoContainerPort,
        WaitFor,
    },
    images::generic::GenericImage,
    runners::AsyncRunner,
    Image,
};

pub(crate) const HOST_INTERNAL_ALIAS: &str = "host.testcontainers.internal";
const SSH_PORT: u16 = 22;

/// Manages the lifetime of the SSH reverse tunnels used to expose host ports.
pub(crate) struct HostPortExposure {
    _sidecar: Box<ContainerAsync<GenericImage>>,
    ssh_handle: Option<client::Handle<HostExposeHandler>>,
    cancel_token: CancellationToken,
}

impl HostPortExposure {
    /// Sets up host port exposure for the provided container request.
    pub(crate) async fn setup<I: Image>(
        container_req: &mut ContainerRequest<I>,
    ) -> Result<Option<Self>, TestcontainersError> {
        // Stage 1: validate the request and derive the parameters we need later.
        let Some(plan) = prepare_host_exposure(container_req)? else {
            return Ok(None);
        };

        // Stage 2: start the SSH sidecar that powers the reverse tunnels.
        let sidecar = spawn_sshd_sidecar(&plan).await?;
        let bridge_ip = sidecar.get_bridge_ip_address().await?;

        container_req
            .hosts
            .insert(HOST_INTERNAL_ALIAS.to_string(), Host::Addr(bridge_ip));

        // Stage 3: establish the SSH session and authenticate against the sidecar.
        let mut ssh_connection = establish_ssh_connection(&sidecar, &plan).await?;

        // Stage 4: request remote port forwards for every requested host port.
        register_requested_ports(&plan.requested_ports, &mut ssh_connection.handle).await?;

        Ok(Some(Self {
            _sidecar: Box::new(sidecar),
            ssh_handle: Some(ssh_connection.handle),
            cancel_token: ssh_connection.cancel_token,
        }))
    }

    pub(crate) fn shutdown(&mut self) {
        self.cancel_token.cancel();

        if let Some(handle) = self.ssh_handle.take() {
            if tokio::runtime::Handle::try_current().is_ok() {
                async_drop::async_drop(async move {
                    if let Err(err) = handle
                        .disconnect(
                            Disconnect::ByApplication,
                            "testcontainers host port exposure cleanup",
                            "",
                        )
                        .await
                    {
                        debug!("ssh disconnect during host exposure cleanup failed: {err}");
                    }
                });
            }
        }
    }
}

struct SshConnection {
    handle: client::Handle<HostExposeHandler>,
    cancel_token: CancellationToken,
}

struct HostExposurePlan {
    requested_ports: Vec<u16>,
    password: String,
    network: Option<String>,
    ssh_username: &'static str,
    ssh_port: u16,
    ssh_max_attempts: u32,
    ssh_retry_delay: Duration,
    ssh_max_retry_delay: Duration,
    ssh_image: &'static str,
    ssh_tag: &'static str,
}

fn prepare_host_exposure<I: Image>(
    container_req: &mut ContainerRequest<I>,
) -> Result<Option<HostExposurePlan>, TestcontainersError> {
    let mut requested_ports = match container_req
        .host_port_exposures()
        .map(|ports| ports.to_vec())
    {
        Some(ports) if !ports.is_empty() => ports,
        _ => return Ok(None),
    };

    // Ensure port list is deduplicated and does not include reserved entries.
    requested_ports.sort_unstable();
    requested_ports.dedup();

    if requested_ports.contains(&0) {
        return Err(other_error(
            "host port exposure requires ports greater than zero (port 0 is invalid)",
        ));
    }

    if requested_ports.contains(&SSH_PORT) {
        return Err(other_error(format!(
            "host port exposure does not support exposing port {} (SSH port is reserved)",
            SSH_PORT
        )));
    }

    if container_req.hosts.contains_key(HOST_INTERNAL_ALIAS) {
        return Err(other_error(
            "host port exposure is not supported when 'host.testcontainers.internal' is already defined",
        ));
    }

    let network = container_req.network().clone();
    if let Some(network_name) = network.as_deref() {
        if network_name == "host" {
            return Err(other_error(
                "host port exposure is not supported with host network mode",
            ));
        }

        if network_name.starts_with("container:") {
            return Err(other_error(
                "host port exposure is not supported with container network mode",
            ));
        }
    }

    #[cfg(feature = "reusable-containers")]
    {
        use crate::ReuseDirective;
        if !matches!(container_req.reuse(), ReuseDirective::Never) {
            return Err(other_error(
                "host port exposure is not supported for reusable containers (due to SSH tunnel conflicts)",
            ));
        }
    }

    let suffix = ULID::from_datetime(std::time::SystemTime::now()).encode();
    let password = format!("tc-{}", suffix.as_str());

    Ok(Some(HostExposurePlan {
        requested_ports,
        password,
        network,
        ssh_username: "root",
        ssh_port: SSH_PORT,
        ssh_max_attempts: 20,
        ssh_retry_delay: Duration::from_millis(100),
        ssh_max_retry_delay: Duration::from_millis(2000),
        ssh_image: "testcontainers/sshd",
        ssh_tag: "1.3.0",
    }))
}

async fn spawn_sshd_sidecar(
    plan: &HostExposurePlan,
) -> Result<ContainerAsync<GenericImage>, TestcontainersError> {
    // Future improvement: swap the SSHD sidecar with a purpose-built container or
    // lightweight SOCKS5 proxy to unlock features like UDP forwarding while keeping
    // host port exposure flexible.
    let mut sshd = GenericImage::new(plan.ssh_image, plan.ssh_tag)
        .with_exposed_port(plan.ssh_port.tcp())
        .with_wait_for(WaitFor::seconds(1))
        .with_env_var("PASSWORD", plan.password.clone());

    if let Some(network) = plan.network.as_deref() {
        sshd = sshd.with_network(network);
    }

    sshd.start().await
}

async fn establish_ssh_connection(
    sidecar: &ContainerAsync<GenericImage>,
    plan: &HostExposurePlan,
) -> Result<SshConnection, TestcontainersError> {
    let ssh_host = sidecar.get_host().await?;
    let ssh_host_port = match ssh_host {
        UrlHost::Domain(_) => sidecar.get_host_port_ipv4(plan.ssh_port.tcp()).await?, // XXX: do we need to handle domain with IPv6 only?
        UrlHost::Ipv4(_) => sidecar.get_host_port_ipv4(plan.ssh_port.tcp()).await?,
        UrlHost::Ipv6(_) => sidecar.get_host_port_ipv6(plan.ssh_port.tcp()).await?,
    };

    let tcp_stream = connect_with_retry(&ssh_host, ssh_host_port, plan).await?;

    let config = client::Config {
        nodelay: true,
        keepalive_interval: Some(Duration::from_secs(10)),
        ..Default::default()
    };
    let cancel_token = CancellationToken::new();
    let handler = HostExposeHandler::new(cancel_token.clone());
    let config = Arc::new(config);

    let mut handle = client::connect_stream(config, tcp_stream, handler)
        .await
        .map_err(TestcontainersError::from)?;

    let auth_result = handle
        .authenticate_password(plan.ssh_username, &plan.password)
        .await
        .map_err(|err| {
            other_error(format!(
                "SSH authentication failed for host port exposure: {err}"
            ))
        })?;

    if !auth_result.success() {
        return Err(other_error(
            "SSH authentication failed for host port exposure - check SSHD container logs and credentials",
        ));
    }

    Ok(SshConnection {
        handle,
        cancel_token,
    })
}

async fn register_requested_ports(
    requested_ports: &[u16],
    ssh_handle: &mut client::Handle<HostExposeHandler>,
) -> Result<(), TestcontainersError> {
    for port in requested_ports {
        let bound_port = ssh_handle
            .tcpip_forward("0.0.0.0", u32::from(*port))
            .await
            .map_err(|err| {
                other_error(format!(
                    "failed to request remote port forwarding for {port}: {err}"
                ))
            })?;

        if bound_port != 0 {
            return Err(other_error(format!(
                "sshd sidecar assigned port {bound_port} instead of requested port {port}",
            )));
        }
    }

    Ok(())
}

impl Drop for HostPortExposure {
    fn drop(&mut self) {
        self.shutdown();
    }
}

async fn connect_with_retry(
    host: &UrlHost,
    port: u16,
    plan: &HostExposurePlan,
) -> Result<TcpStream, TestcontainersError> {
    let host_str = host.to_string();
    let mut attempts = 0;
    let mut delay = plan.ssh_retry_delay;

    loop {
        match TcpStream::connect((host_str.as_str(), port)).await {
            Ok(stream) => {
                if let Err(err) = stream.set_nodelay(true) {
                    return Err(other_error(format!(
                        "failed to configure ssh tcp stream: {err}"
                    )));
                }
                return Ok(stream);
            }
            Err(err) if attempts < plan.ssh_max_attempts => {
                attempts += 1;
                sleep(delay).await;
                delay = std::cmp::min(delay * 2, plan.ssh_max_retry_delay);
                trace!(
                    "waiting for sshd sidecar to be reachable at {host}:{port}: {err}",
                    host = host_str.as_str()
                );
            }
            Err(err) => {
                return Err(other_error(format!(
                    "failed to connect to sshd sidecar at {host}:{port}: {err}",
                    host = host_str.as_str()
                )))
            }
        }
    }
}

fn other_error(message: impl Into<String>) -> TestcontainersError {
    TestcontainersError::other(message.into())
}

#[derive(Clone)]
struct HostExposeHandler {
    cancel_token: CancellationToken,
}

impl HostExposeHandler {
    fn new(cancel_token: CancellationToken) -> Self {
        Self { cancel_token }
    }

    async fn prepare_forwarding_stream(
        &self,
        remote_port: u16,
        connected_address: &str,
        originator_address: &str,
        originator_port: u32,
    ) -> Result<TcpStream, HostExposeError> {
        let stream = TcpStream::connect(("localhost", remote_port)).await.map_err(|err| {
            HostExposeError::Exposure(other_error(format!(
                "failed to connect to host port {remote_port} for exposure tunnel (via {connected_address} from {originator_address}:{originator_port}): {err}",
            )))
        })?;

        stream.set_nodelay(true).map_err(|err| {
            HostExposeError::Exposure(other_error(format!(
                "failed to configure tcp stream for host exposure port {remote_port}: {err}",
            )))
        })?;

        Ok(stream)
    }

    async fn forward_connection(
        &self,
        channel: Channel<client::Msg>,
        mut stream: TcpStream,
        port: u16,
    ) -> io::Result<()> {
        if self.cancel_token.is_cancelled() {
            return Ok(());
        }

        let mut channel_stream = channel.into_stream();
        let cancellation = self.cancel_token.cancelled();
        tokio::pin!(cancellation);

        tokio::select! {
            result = copy_bidirectional(&mut channel_stream, &mut stream) => {
                result?;
            }
            _ = &mut cancellation => {}
        }

        if let Err(err) = stream.shutdown().await {
            trace!(
                "failed to shutdown tcp stream after host exposure proxy for port {port}: {err}"
            );
        }

        Ok(())
    }

    fn start_forward_connection(self, channel: Channel<client::Msg>, stream: TcpStream, port: u16) {
        if self.cancel_token.is_cancelled() {
            return;
        }

        tokio::spawn(async move {
            if let Err(err) = self.forward_connection(channel, stream, port).await {
                debug!("host port exposure proxy for remote port {port} ended with error: {err}");
            }
        });
    }
}

impl client::Handler for HostExposeHandler {
    type Error = HostExposeError;

    fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> impl future::Future<Output = Result<bool, Self::Error>> + Send {
        // skip server key verification for the ephemeral SSHD sidecar
        future::ready(Ok(true))
    }

    fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut client::Session,
    ) -> impl future::Future<Output = Result<(), Self::Error>> + Send {
        let client = self.clone();
        let connected_address = connected_address.to_string();
        let originator_address = originator_address.to_string();
        async move {
            if client.cancel_token.is_cancelled() {
                return Ok(());
            }

            let remote_port = u16::try_from(connected_port)
                .expect("forwarded connection reported port outside u16 range");

            let stream = client
                .prepare_forwarding_stream(
                    remote_port,
                    connected_address.as_str(),
                    originator_address.as_str(),
                    originator_port,
                )
                .await?;

            client.start_forward_connection(channel, stream, remote_port);

            Ok(())
        }
    }
}

#[derive(Debug)]
enum HostExposeError {
    Ssh(russh::Error),
    Exposure(TestcontainersError),
}

impl From<russh::Error> for HostExposeError {
    fn from(err: russh::Error) -> Self {
        Self::Ssh(err)
    }
}

impl From<TestcontainersError> for HostExposeError {
    fn from(err: TestcontainersError) -> Self {
        Self::Exposure(err)
    }
}

impl From<HostExposeError> for TestcontainersError {
    fn from(err: HostExposeError) -> Self {
        match err {
            HostExposeError::Ssh(err) => other_error(format!("ssh error: {err}")),
            HostExposeError::Exposure(err) => err,
        }
    }
}
