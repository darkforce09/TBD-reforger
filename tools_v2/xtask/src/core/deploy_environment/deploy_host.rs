//! The deploy host named by `TBD_SSH_HOST`: its ssh destination, the deploy user's folders and
//! its IPv4 address.
//!
//! **Role:** parses `user@host` or `host`, and answers what the commands derive from it.
//!
//! **Position:** built by [`super::DeployEnvironment::deploy_host`]; read by the deploys, the
//! staging bootstrap, the log reader, the join probes and [`super::DeployHostFolder`].
//!
//! **Signals & state:** none. [`first_ipv4_address`] asks the system resolver (`/etc/hosts`, DNS,
//! mDNS through the host's name service) unless the name is already an IPv4 literal.
//!
//! **Invariants:** a parsed host holds no whitespace, at most one `@`, no empty part and no
//! leading `-`, so ssh can never read it as an option; the deploy user's home is
//! `/home/<user>`, and a host named without a user has no home to default folders under.

use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};

/// `TBD_SSH_HOST`, split into its optional user and its host name or address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployHost {
    user: Option<String>,
    host: String,
}

impl DeployHost {
    /// Parses `user@host` or `host`, or says what is wrong with the value.
    pub fn parse(value: &str) -> Result<DeployHost, String> {
        if value.chars().any(char::is_whitespace) {
            return Err("holds whitespace; write user@host or host".to_string());
        }
        if value.starts_with('-') {
            return Err("starts with `-`, which ssh would read as an option".to_string());
        }
        let (user, host) = match value.split_once('@') {
            Some((_, host)) if host.contains('@') => {
                return Err("holds a second `@`; write user@host or host".to_string());
            }
            Some(("", _)) => return Err("names no user before `@`".to_string()),
            Some((user, host)) => (Some(user.to_string()), host),
            None => (None, value),
        };
        if host.is_empty() {
            return Err("names no host".to_string());
        }
        if host.starts_with('-') {
            return Err(
                "names a host starting with `-`, which ssh would read as an option".to_string(),
            );
        }
        Ok(DeployHost {
            user,
            host: host.to_string(),
        })
    }

    /// The deploy user, when the value names one.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// The host name or address, without the user.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// What ssh and rsync connect to: `user@host`, or `host` alone.
    pub fn ssh_destination(&self) -> String {
        match &self.user {
            Some(user) => format!("{user}@{}", self.host),
            None => self.host.clone(),
        }
    }

    /// `/home/<user>`, the folder the remote defaults sit under.
    pub fn home_directory(&self) -> Option<String> {
        self.user().map(|user| format!("/home/{user}"))
    }

    /// `/home/<user>/tbd`, the one folder `deploy website` may rsync `--delete` into.
    pub fn tbd_folder(&self) -> Option<String> {
        self.home_directory().map(|home| format!("{home}/tbd"))
    }

    /// [`first_ipv4_address`] of the host.
    pub fn resolve_ipv4(&self) -> Result<Ipv4Addr, String> {
        first_ipv4_address(&self.host)
    }
}

/// The first IPv4 address `host` resolves to from this machine. An IPv4 literal is returned
/// without a lookup; a name that resolves to IPv6 addresses only is an error, since the game
/// server's room and the UDP probes need IPv4.
pub fn first_ipv4_address(host: &str) -> Result<Ipv4Addr, String> {
    if let Ok(address) = host.parse::<Ipv4Addr>() {
        return Ok(address);
    }
    let addresses: Vec<SocketAddr> = (host, 0)
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .collect();
    first_ipv4_of(addresses.iter().copied()).ok_or_else(|| {
        if addresses.is_empty() {
            return "the lookup returned no address".to_string();
        }
        let listed: Vec<String> = addresses.iter().map(|a| a.ip().to_string()).collect();
        format!("it resolves to IPv6 only: {}", listed.join(", "))
    })
}

/// The first IPv4 address in resolver order, which on many systems lists IPv6 first.
fn first_ipv4_of(addresses: impl IntoIterator<Item = SocketAddr>) -> Option<Ipv4Addr> {
    addresses.into_iter().find_map(|address| match address {
        SocketAddr::V4(v4) => Some(*v4.ip()),
        SocketAddr::V6(_) => None,
    })
}

#[cfg(test)]
#[path = "tests/deploy_host/tests.rs"]
mod tests;
