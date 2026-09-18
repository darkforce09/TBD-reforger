//! Resolving "which client is this request from" for the rate limiters.
//!
//! Both limiter tiers key on one address, produced here. The address is the direct connection
//! peer, except when that peer is a configured trusted proxy — then the `X-Forwarded-For` chain
//! is read, under three rules that keep the header from becoming a bypass:
//!
//! 1. **The immediate peer must be a trusted proxy.** A client connecting directly gets its own
//!    connection's address no matter what it sends, so the header is never a spoofing tool.
//! 2. **The hop taken is the rightmost untrusted one**, never the leftmost. A proxy *appends* the
//!    address it observed, so the rightmost entry is the proxy's own measurement and everything
//!    left of it is client-supplied text. Taking the leftmost would let any client mint a fresh
//!    bucket per request by varying one header.
//! 3. **An empty trusted-proxy list ignores the header entirely**, which is the shipped default.
//!
//! Anything the chain cannot answer honestly — no header, an unparseable hop, `0.0.0.0`, a chain
//! made entirely of trusted proxies — falls back to the peer, i.e. to a bucket shared by
//! everyone behind that proxy. Fail-closed here means "limit them together", never "let them
//! through": a forgeable key is strictly worse than a shared one.
//!
//! Trusting a proxy is trusting it to report its clients honestly. That assumption is the
//! feature; it cannot be verified from inside this process.

use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, Request};
use axum::http::HeaderMap;

use crate::core::configuration::proxy_network::ProxyNet;

/// How far back through `X-Forwarded-For` this will look before giving up and using the peer.
///
/// The scan stops at the first untrusted hop, so a real chain costs one iteration. The cap only
/// binds on a chain of 64+ *trusted* hops, which no deployment has and a client cannot manufacture
/// (it would have to make every entry match the trust list); giving up there costs the shared
/// bucket, not a bypass. It exists so a pathological header cannot buy unbounded work per request.
const MAX_FORWARDED_HOPS: usize = 64;

/// The forwarding header this reads, lowercase because `HeaderMap` lookups are case-insensitive
/// on a lowercase key.
///
/// Named here rather than taken from `http::header` because `X-Forwarded-For` is a de-facto
/// standard, not a registered one, and the crate has no constant for it.
const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// The client this request is attributable to, or `None` when it has no client.
///
/// The address is the direct connection peer from [`ConnectInfo`], **unless** that peer is a
/// configured trusted proxy, in which case the chain in `X-Forwarded-For` is consulted — see the
/// module header for the three rules and why the rightmost untrusted hop is the only safe choice.
/// With `trusted` empty (the default) this is simply the peer, always.
///
/// `None` means the request did not arrive over a socket — there is no `ConnectInfo` extension at
/// all, or the peer is the unspecified address. **`0.0.0.0` / `::` are not client addresses**; a
/// packet cannot originate from them, so a request bearing one was synthesised in-process (a
/// `tower::ServiceExt::oneshot` call in a test, a future in-process mount). Attributing such a
/// request to a durable per-client bucket would file every in-process caller under one key and
/// leave a persistent row for a client that does not exist.
///
/// The peer is canonicalised (`::ffff:1.2.3.4` → `1.2.3.4`) so one client is one bucket whether the
/// listener is dual-stack or not — otherwise the same client keys two different buckets depending
/// on how the socket was opened, and `TRUSTED_PROXIES=127.0.0.1` would match nothing on a
/// dual-stack listener.
///
/// The production path always has a real peer: `bin/api.rs` serves with
/// `into_make_service_with_connect_info::<SocketAddr>()`, which installs the extension for every
/// accepted connection. That is not a comment relying on good behaviour —
/// `durable_rate_limit::api_binary_still_installs_connect_info` reads `src/bin/api.rs` and fails if it
/// stops doing so, because the day it does, this function starts returning `None` in production
/// and both tiers quietly stop distinguishing clients.
pub(super) fn client_ip(req: &Request, trusted: &[ProxyNet]) -> Option<IpAddr> {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()?
        .0
        .ip()
        .to_canonical();
    if peer.is_unspecified() {
        return None;
    }
    // Rule 1: the header is only evidence when the hop that handed it to us is one we trust.
    // A direct client's header is not evidence of anything.
    if !trusted.iter().any(|net| net.contains(peer)) {
        return Some(peer);
    }
    Some(forwarded_client(req.headers(), trusted).unwrap_or(peer))
}

/// The rightmost `X-Forwarded-For` hop that is **not** a trusted proxy, or `None` to fall back.
///
/// Reads right-to-left because that is the direction trust flows: the last entry was written by
/// the proxy we just accepted the connection from, the one before it by the proxy before that, and
/// everything left of the first untrusted entry is client-supplied text. The first entry that is
/// not a trusted proxy is therefore the furthest hop anyone in the chain actually *observed*.
///
/// `None` — meaning "use the peer" — for every case where the chain cannot be read honestly:
///
/// * no `X-Forwarded-For` header at all (a proxy that does not set one);
/// * a hop that is not an address (`unknown`, an RFC 7239 obfuscated token, junk). The scan
///   **stops** there rather than skipping it: skipping would let a client shift which entry is
///   read by injecting one unparseable value, which is the leftmost-hop bug wearing a hat;
/// * `0.0.0.0` / `::`, which is not a source address any packet can carry;
/// * a chain made entirely of trusted proxies, or longer than [`MAX_FORWARDED_HOPS`].
///
/// Only `X-Forwarded-For` is read. RFC 7239 `Forwarded` is not consulted because the deployed
/// proxy does not send it, and a header nothing writes is a parser nothing tests.
fn forwarded_client(headers: &HeaderMap, trusted: &[ProxyNet]) -> Option<IpAddr> {
    let hops: Vec<&str> = headers
        .get_all(X_FORWARDED_FOR)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .collect();
    for hop in hops.iter().rev().take(MAX_FORWARDED_HOPS) {
        let ip = parse_forwarded_hop(hop)?;
        if ip.is_unspecified() {
            return None;
        }
        if !trusted.iter().any(|net| net.contains(ip)) {
            return Some(ip);
        }
    }
    None
}

/// One `X-Forwarded-For` entry as an address.
///
/// Accepts the bare address every proxy in this deployment writes, plus the `addr:port` and
/// `[v6]:port` spellings some proxies emit, plus a bracketed IPv6 with no port. Anything else is
/// `None`, which stops the scan.
fn parse_forwarded_hop(hop: &str) -> Option<IpAddr> {
    if let Ok(ip) = hop.parse::<IpAddr>() {
        return Some(ip.to_canonical());
    }
    if let Ok(sock) = hop.parse::<SocketAddr>() {
        return Some(sock.ip().to_canonical());
    }
    hop.strip_prefix('[')?
        .strip_suffix(']')?
        .parse::<IpAddr>()
        .ok()
        .map(|ip| ip.to_canonical())
}

#[cfg(test)]
#[path = "tests/client_identity.rs"]
mod tests;
