use std::{fmt::Debug, future::Future, pin::Pin, sync::Arc, time::Duration};

use bytes::Bytes;
use url::{Host, Url};

use crate::{
    core::{client::Client, error::WaitContainerError, wait::WaitStrategy, ContainerPort},
    ContainerAsync, Image, TestcontainersError,
};

/// Error type for waiting for container readiness based on HTTP response.
#[derive(Debug, thiserror::Error)]
pub enum HttpWaitError {
    #[error("container has no exposed ports")]
    NoExposedPortsForHttpWait,
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

/// Represents a strategy for waiting for a certain HTTP response.
#[derive(Clone)]
pub struct HttpWaitStrategy {
    client: Option<reqwest::Client>,
    path: String,
    port: Option<ContainerPort>,
    method: reqwest::Method,
    headers: reqwest::header::HeaderMap,
    body: Option<Bytes>,
    auth: Option<Auth>,
    use_tls: bool,
    response_matcher: Option<ResponseMatcher>,
    poll_interval: Duration,
}

type ResponseMatcher = Arc<
    dyn Fn(reqwest::Response) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync + 'static,
>;

#[derive(Debug, Clone)]
enum Auth {
    Basic { username: String, password: String },
    Bearer(String),
}

impl HttpWaitStrategy {
    /// Create a new `HttpWaitStrategy` for the given resource path (using GET method by default).
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            client: None,
            path: path.into(),
            port: None,
            method: reqwest::Method::GET,
            headers: Default::default(),
            body: None,
            auth: None,
            use_tls: false,
            response_matcher: None,
            poll_interval: Duration::from_millis(100),
        }
    }

    /// Set the port to be used for the request.
    ///
    /// It will use mapped host port for the passed container port. By default, first exposed port is used.
    pub fn with_port(mut self, port: ContainerPort) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the custom client for the request.
    ///
    /// Allows to customize the client, enabling features like TLS, accept_invalid_certs, proxies, etc.
    /// If you need to use particular features of `reqwest`, just add `reqwest` to your dependencies with desired features enabled.
    /// After that, you can create a client with the desired configuration and pass it to the wait strategy.
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Set method for the request.
    pub fn with_method(mut self, method: reqwest::Method) -> Self {
        self.method = method;
        self
    }

    /// Add a header to the request.
    pub fn with_header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: reqwest::header::IntoHeaderName,
        V: Into<reqwest::header::HeaderValue>,
    {
        self.headers.insert(key, value.into());
        self
    }

    /// Set the body for the request.
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the basic auth for the request.
    /// Overwrites any previously set Authorization header.
    pub fn with_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.auth = Some(Auth::Basic {
            username: username.into(),
            password: password.into(),
        });
        self
    }

    /// Set the bearer token for the request.
    /// Overwrites any previously set Authorization header.
    pub fn with_bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(Auth::Bearer(token.into()));
        self
    }

    /// Use TLS for the request.
    ///
    /// This will use `https` scheme for the request. TLS configuration can be customized using the [`HttpWaitStrategy::with_client`].
    pub fn with_tls(mut self) -> Self {
        self.use_tls = true;
        self
    }

    /// Set the poll interval for the wait strategy.
    ///
    /// This is the time to wait between each poll for the expected condition to be met.
    pub fn with_poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Wait for expected status code.
    /// Shortcut for `with_response_matcher(|response| response.status() == status)`.
    pub fn with_expected_status_code(self, status: impl Into<u16>) -> Self {
        let status = status.into();
        self.with_response_matcher(move |response| response.status().as_u16() == status)
    }

    /// Wait for a response that matches the given matcher function.
    /// Use [`HttpWaitStrategy::with_response_matcher_async`] for async matcher functions.
    ///
    /// Matcher function should return `true` if the response is expected, `false` otherwise.
    pub fn with_response_matcher<Matcher>(self, matcher: Matcher) -> Self
    where
        Matcher: Fn(reqwest::Response) -> bool + Send + Sync + 'static,
    {
        let matcher = Arc::new(matcher);
        self.with_response_matcher_async(move |response| {
            let matcher = matcher.clone();
            async move { matcher(response) }
        })
    }

    /// Wait for a response that matches the result of given matcher function.
    /// This is an async version of [`HttpWaitStrategy::with_response_matcher`],
    ///     useful when the matcher function needs to perform additional async operations (e.g. body reading to check response content).
    ///
    /// Matcher function should return `true` if the response is expected, `false` otherwise.
    pub fn with_response_matcher_async<Matcher, Out>(mut self, matcher: Matcher) -> Self
    where
        Matcher: Fn(reqwest::Response) -> Out,
        Matcher: Send + Sync + 'static,
        for<'a> Out: Future<Output = bool> + Send + 'a,
    {
        self.response_matcher = Some(Arc::new(move |resp| Box::pin(matcher(resp))));
        self
    }

    pub(crate) fn response_matcher(&self) -> Option<ResponseMatcher> {
        self.response_matcher.clone()
    }

    pub(crate) fn into_request(
        self,
        base_url: &Url,
    ) -> Result<reqwest::RequestBuilder, HttpWaitError> {
        let client = self.client.unwrap_or_default();
        let url = base_url.join(&self.path).map_err(HttpWaitError::from)?;
        let mut request = client.request(self.method, url).headers(self.headers);

        if let Some(body) = self.body {
            request = request.body(body);
        }

        if let Some(auth) = self.auth {
            match auth {
                Auth::Basic { username, password } => {
                    request = request.basic_auth(username, Some(password));
                }
                Auth::Bearer(token) => {
                    request = request.bearer_auth(token);
                }
            }
        }

        Ok(request)
    }
}

impl WaitStrategy for HttpWaitStrategy {
    async fn wait_until_ready<I: Image>(
        self,
        _client: &Client,
        container: &ContainerAsync<I>,
    ) -> crate::core::error::Result<()> {
        let host = container.get_host().await?;
        let container_port = self
            .port
            .or_else(|| container.image().expose_ports().first().copied())
            .ok_or(WaitContainerError::from(
                HttpWaitError::NoExposedPortsForHttpWait,
            ))?;

        let host_port = match host {
            Host::Domain(ref domain) => match container.get_host_port_ipv4(container_port).await {
                Ok(port) => port,
                Err(_) => {
                    log::debug!("IPv4 port not found for domain: {domain}, checking for IPv6");
                    container.get_host_port_ipv6(container_port).await?
                }
            },
            Host::Ipv4(_) => container.get_host_port_ipv4(container_port).await?,
            Host::Ipv6(_) => container.get_host_port_ipv6(container_port).await?,
        };

        let scheme = if self.use_tls { "https" } else { "http" };
        let base_url = Url::parse(&format!("{scheme}://{host}:{host_port}"))
            .map_err(HttpWaitError::from)
            .map_err(WaitContainerError::from)?;

        loop {
            let Some(matcher) = self.response_matcher() else {
                return Err(TestcontainersError::other(format!(
                    "No response matcher provided for HTTP wait strategy: {self:?}"
                )));
            };
            let result = self
                .clone()
                .into_request(&base_url)
                .map_err(WaitContainerError::from)?
                .send()
                .await;

            match result {
                Ok(response) => {
                    if matcher(response).await {
                        log::debug!("HTTP response condition met");
                        break;
                    } else {
                        log::debug!("HTTP response condition not met");
                    }
                }
                Err(err) => {
                    log::debug!("Error while waiting for HTTP response: {}", err);
                }
            }
            tokio::time::sleep(self.poll_interval).await;
        }
        Ok(())
    }
}

impl Debug for HttpWaitStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpWaitStrategy")
            .field("path", &self.path)
            .field("method", &self.method)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("auth", &self.auth)
            .finish()
    }
}
