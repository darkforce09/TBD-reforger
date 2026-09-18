//! Unit coverage for trusted-proxy client resolution: the spoof cases, the rightmost-hop rule,
//! the fall-backs, and the hop spellings a real proxy writes.

use super::*;

fn nets(entries: &[&str]) -> Vec<ProxyNet> {
    entries
        .iter()
        .map(|e| ProxyNet::parse(e).unwrap_or_else(|why| panic!("{e:?}: {why}")))
        .collect()
}

/// A request as the router sees one: an optional `ConnectInfo` peer and zero or more
/// `X-Forwarded-For` **header lines** (a chain may legally arrive split across several).
fn req_from(peer: Option<&str>, forwarded: &[&str]) -> Request {
    let mut b = axum::http::Request::builder().uri("/api/v1/auth/refresh");
    for v in forwarded {
        b = b.header(X_FORWARDED_FOR, *v);
    }
    let mut req = b.body(axum::body::Body::empty()).expect("request");
    if let Some(peer) = peer {
        let ip: IpAddr = peer.parse().expect("peer address");
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(ip, 51_000)));
    }
    req
}

fn resolved(peer: Option<&str>, forwarded: &[&str], trusted: &[&str]) -> Option<IpAddr> {
    client_ip(&req_from(peer, forwarded), &nets(trusted))
}

fn addr(s: &str) -> Option<IpAddr> {
    Some(s.parse().expect("address"))
}

/// **The default deployment.** With no trusted proxy configured the header is not read at all.
#[test]
fn an_empty_trust_list_ignores_the_header_entirely() {
    assert_eq!(resolved(Some("203.0.113.9"), &[], &[]), addr("203.0.113.9"));
    assert_eq!(
        resolved(Some("203.0.113.9"), &["198.51.100.10"], &[]),
        addr("203.0.113.9"),
        "an unconfigured deployment must not read X-Forwarded-For"
    );
    // Including a header that claims to be the proxy itself.
    assert_eq!(
        resolved(Some("127.0.0.1"), &["198.51.100.10"], &[]),
        addr("127.0.0.1")
    );
}

/// **The spoof.** A client that connects directly cannot hand itself another identity, no
/// matter what it sends — the peer is not a trusted proxy, so the header is not evidence.
#[test]
fn a_forged_header_from_an_untrusted_peer_is_ignored() {
    for forged in [
        "198.51.100.10",
        "127.0.0.1",
        "203.0.113.9, 198.51.100.10",
        "10.0.0.1",
    ] {
        assert_eq!(
            resolved(Some("203.0.113.9"), &[forged], &["127.0.0.1"]),
            addr("203.0.113.9"),
            "forged header {forged:?} from an untrusted peer changed the key"
        );
    }
}

/// **The rule that keeps this from being a vulnerability.** A proxy appends the address it
/// observed, so the rightmost hop is the proxy's measurement and everything left of it is the
/// client's text. A leftmost implementation returns `9.9.9.9` here — a fresh bucket per
/// request, for free.
#[test]
fn the_rightmost_untrusted_hop_is_taken_not_the_leftmost() {
    assert_eq!(
        resolved(
            Some("127.0.0.1"),
            &["9.9.9.9, 198.51.100.10"],
            &["127.0.0.1"]
        ),
        addr("198.51.100.10")
    );
    // Varying the forged prefix cannot vary the answer.
    assert_eq!(
        resolved(
            Some("127.0.0.1"),
            &["203.0.113.7, 198.51.100.10"],
            &["127.0.0.1"]
        ),
        addr("198.51.100.10")
    );
}

/// Two proxies in front of the API: both are skipped, the client behind them is taken.
#[test]
fn trusted_hops_are_skipped_from_the_right() {
    assert_eq!(
        resolved(
            Some("127.0.0.1"),
            &["198.51.100.10, 10.1.1.1, 10.2.2.2"],
            &["127.0.0.1", "10.0.0.0/8"]
        ),
        addr("198.51.100.10")
    );
}

/// A chain split across several header lines is one chain, in order.
#[test]
fn multiple_header_lines_are_one_chain() {
    assert_eq!(
        resolved(
            Some("127.0.0.1"),
            &["9.9.9.9", "198.51.100.10"],
            &["127.0.0.1"]
        ),
        addr("198.51.100.10")
    );
}

/// Every chain that cannot be read honestly falls back to the peer — the shared bucket.
/// Fail-closed here is "limited together", never "not limited".
#[test]
fn an_unusable_chain_falls_back_to_the_peer() {
    let peer = addr("127.0.0.1");
    // No header at all.
    assert_eq!(resolved(Some("127.0.0.1"), &[], &["127.0.0.1"]), peer);
    for chain in [
        "",                         // empty value
        "unknown",                  // RFC 7239 token
        "_hidden",                  // obfuscated identifier
        "198.51.100.10, not-an-ip", // junk to the RIGHT of a real address: stop, do not
        "198.51.100.10, unknown",   // skip past it — skipping is the leftmost bug again
        "0.0.0.0",                  // not a source address
        "198.51.100.10, 0.0.0.0",   // …including at the end of a chain
        "127.0.0.1",                // nothing but trusted proxies
        "127.0.0.1, 127.0.0.1",     //
        "example.com",              // a name, not an address
    ] {
        assert_eq!(
            resolved(Some("127.0.0.1"), &[chain], &["127.0.0.1"]),
            peer,
            "chain {chain:?} should have fallen back to the peer"
        );
    }
}

/// A request with no socket has no client, header or not. Acquiring one from a header would
/// hand every in-process caller a forgeable identity and file a durable bucket for it.
#[test]
fn a_peerless_request_has_no_client_even_with_a_header() {
    assert_eq!(resolved(None, &["198.51.100.10"], &["127.0.0.1"]), None);
    assert_eq!(resolved(None, &[], &[]), None);
    // The unspecified peer is the same case: synthesised in-process.
    assert_eq!(
        resolved(Some("0.0.0.0"), &["198.51.100.10"], &["127.0.0.1"]),
        None
    );
}

/// The spellings a proxy might actually write, all landing on the same client.
#[test]
fn hop_spellings_parse_to_canonical_addresses() {
    assert_eq!(parse_forwarded_hop("198.51.100.10"), addr("198.51.100.10"));
    assert_eq!(
        parse_forwarded_hop("198.51.100.10:41234"),
        addr("198.51.100.10")
    );
    assert_eq!(
        parse_forwarded_hop("::ffff:198.51.100.10"),
        addr("198.51.100.10"),
        "an IPv4-mapped hop is that IPv4 client, not a separate bucket"
    );
    assert_eq!(parse_forwarded_hop("2001:db8::1"), addr("2001:db8::1"));
    assert_eq!(parse_forwarded_hop("[2001:db8::1]"), addr("2001:db8::1"));
    assert_eq!(
        parse_forwarded_hop("[2001:db8::1]:443"),
        addr("2001:db8::1")
    );
    for junk in [
        "",
        "unknown",
        "_secret",
        "example.com",
        "1.2.3",
        "1.2.3.4.5",
    ] {
        assert_eq!(
            parse_forwarded_hop(junk),
            None,
            "{junk:?} is not an address"
        );
    }
}

/// A dual-stack listener's mapped peer is the same client as the plain one — one bucket, and
/// `TRUSTED_PROXIES=127.0.0.1` still recognises the proxy.
#[test]
fn a_mapped_peer_is_canonicalised_before_anything_else() {
    assert_eq!(
        resolved(Some("::ffff:203.0.113.9"), &[], &[]),
        addr("203.0.113.9")
    );
    assert_eq!(
        resolved(Some("::ffff:127.0.0.1"), &["198.51.100.10"], &["127.0.0.1"]),
        addr("198.51.100.10"),
        "a mapped loopback peer is still the trusted proxy"
    );
}

/// The hop cap binds only on a chain of trusted proxies, and giving up costs the shared
/// bucket rather than a bypass.
#[test]
fn an_absurdly_long_trusted_chain_falls_back_to_the_peer() {
    let mut chain = vec!["198.51.100.10".to_string()];
    chain.extend(std::iter::repeat_n(
        "10.0.0.2".to_string(),
        MAX_FORWARDED_HOPS + 1,
    ));
    let joined = chain.join(", ");
    assert_eq!(
        resolved(Some("10.0.0.1"), &[&joined], &["10.0.0.0/8"]),
        addr("10.0.0.1"),
        "a chain longer than the cap must fall back to the peer, not read past it"
    );
    // One hop under the cap, the client is still found.
    let short = std::iter::repeat_n("10.0.0.2".to_string(), MAX_FORWARDED_HOPS - 1)
        .collect::<Vec<_>>()
        .join(", ");
    let joined = format!("198.51.100.10, {short}");
    assert_eq!(
        resolved(Some("10.0.0.1"), &[&joined], &["10.0.0.0/8"]),
        addr("198.51.100.10")
    );
}
