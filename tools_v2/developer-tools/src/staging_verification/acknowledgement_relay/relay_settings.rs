//! The relay's addresses and hold time, and the loopback rule every address must pass.
//!
//! - **Role:** turns the `serve` flags into [`RelaySettings`]: a loopback listen address, an http
//!   upstream origin on a loopback host, the control socket path and the hold time of a withheld
//!   answer.
//! - **Position:** under [`super`]; the command line builds the settings before anything binds,
//!   and [`super::relay`] builds its listener and its upstream client from them.
//! - **Signals & state:** none; pure functions.
//! - **Invariants:**
//!   - The listen address is an IP literal in `127.0.0.0/8` or `::1`: never a name or a wildcard.
//!   - The upstream is `http://` on a loopback IP literal or on `localhost`, which is pinned to
//!     `127.0.0.1` rather than resolved, and names no user, path, query or fragment.
//!   - [`DEFAULT_WITHHOLD`] outlasts [`AGENT_REQUEST_TIMEOUT`], so the agent gives up first.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::Url;

/// The host agent's whole-request timeout on every call to the command ledger.
pub const AGENT_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a withheld answer is held before its connection closes: the agent's timeout and a
/// ten-second margin.
pub const DEFAULT_WITHHOLD: Duration = Duration::from_secs(30);

/// Where the relay listens, where it forwards, where it is controlled, and how long it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaySettings {
    /// The loopback address the host agent sends its requests to.
    pub listen: SocketAddr,
    /// The API origin every exchange is forwarded to.
    pub upstream: UpstreamOrigin,
    /// The path of the control socket the relay creates.
    pub control_socket: PathBuf,
    /// How long a withheld answer is held before its connection closes.
    pub withhold: Duration,
}

impl RelaySettings {
    /// Settings from the `serve` flags, holding a withheld answer for [`DEFAULT_WITHHOLD`].
    ///
    /// # Errors
    ///
    /// `listen` is not a loopback socket address, or `upstream` is not an http origin on a
    /// loopback host.
    pub fn from_flags(listen: &str, upstream: &str, control_socket: PathBuf) -> Result<Self> {
        Ok(Self {
            listen: loopback_listen_address(listen)?,
            upstream: UpstreamOrigin::parse(upstream)?,
            control_socket,
            withhold: DEFAULT_WITHHOLD,
        })
    }
}

/// `text` as a socket address whose IP is a loopback address.
///
/// # Errors
///
/// `text` is not an IP address and port, or its IP is not a loopback address.
pub fn loopback_listen_address(text: &str) -> Result<SocketAddr> {
    let address: SocketAddr = text.parse().with_context(|| {
        format!("--listen {text} is not an IP address and port, such as 127.0.0.1:18085")
    })?;
    if !address.ip().is_loopback() {
        bail!(
            "--listen {text} is not a loopback address: the relay listens only on 127.0.0.0/8 or ::1"
        );
    }
    Ok(address)
}

/// The API origin the relay forwards to, and the loopback address it connects to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamOrigin {
    /// `http://<host>:<port>`, without a trailing slash.
    origin: String,
    /// The host as the origin spells it: a loopback IP literal or `localhost`.
    host: String,
    /// The loopback address the relay connects to.
    address: SocketAddr,
}

impl UpstreamOrigin {
    /// `text` as an http origin on a loopback host.
    ///
    /// # Errors
    ///
    /// `text` is not a URL, is not http, names a user, a path, a query or a fragment, or its host
    /// is neither a loopback IP literal nor `localhost`.
    pub fn parse(text: &str) -> Result<Self> {
        let url = Url::parse(text).with_context(|| {
            format!("--upstream {text} is not a URL, such as http://127.0.0.1:8080")
        })?;
        if url.scheme() != "http" {
            bail!(
                "--upstream {text} is not http: the relay forwards only to http on a loopback host"
            );
        }
        if !url.username().is_empty() || url.password().is_some() {
            // The flag is not repeated: its user part may carry a password.
            bail!("--upstream names a user: the relay forwards to a bare origin");
        }
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            bail!(
                "--upstream {text} has a path, a query or a fragment: the relay forwards to a bare origin"
            );
        }
        let host = url.host_str().unwrap_or_default().to_string();
        let ip = if host == "localhost" {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        } else {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<IpAddr>()
                .ok()
                .filter(IpAddr::is_loopback)
                .with_context(|| {
                    format!("--upstream {text} is not on a loopback host: the relay forwards only to 127.0.0.0/8, ::1 or localhost")
                })?
        };
        let port = url.port_or_known_default().unwrap_or(80);
        Ok(Self {
            origin: format!("http://{host}:{port}"),
            host,
            address: SocketAddr::new(ip, port),
        })
    }

    /// The origin, `http://<host>:<port>`.
    pub fn as_str(&self) -> &str {
        &self.origin
    }

    /// The host as the origin spells it.
    pub(super) fn host(&self) -> &str {
        &self.host
    }

    /// The loopback address the relay connects to.
    pub(super) fn address(&self) -> SocketAddr {
        self.address
    }

    /// The upstream URL of a request whose target is `path_and_query`.
    ///
    /// # Errors
    ///
    /// `path_and_query` does not start with `/`, or does not form a URL with the origin.
    pub(super) fn target(&self, path_and_query: &str) -> Result<Url> {
        if !path_and_query.starts_with('/') {
            bail!("the request target {path_and_query} is not a path");
        }
        Url::parse(&format!("{}{path_and_query}", self.origin))
            .with_context(|| format!("the request target {path_and_query} does not form a URL"))
    }
}

#[cfg(test)]
#[path = "tests/relay_settings_tests.rs"]
mod tests;
