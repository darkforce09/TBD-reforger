use super::*;

fn net(entry: &str) -> ProxyNet {
    ProxyNet::parse(entry).unwrap_or_else(|e| panic!("{entry:?} should parse: {e}"))
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address")
}

/// A bare address is that host and **nothing else**. The failure this pins is the widening
/// one: reading `10.0.0.1` as "the 10.0.0.0/8 this address sits in" would trust 16 million
/// hosts on the strength of one line.
#[test]
fn a_bare_address_is_a_single_host() {
    let n = net("127.0.0.1");
    assert!(n.contains(ip("127.0.0.1")));
    assert!(!n.contains(ip("127.0.0.2")));
    assert!(!n.contains(ip("127.1.0.1")));

    let six = net("::1");
    assert!(six.contains(ip("::1")));
    assert!(!six.contains(ip("::2")));
}

/// CIDR membership, including a prefix that does not land on a byte boundary — the case a
/// byte-wise comparison gets wrong in the permissive direction.
#[test]
fn cidr_membership_is_bitwise() {
    let n = net("10.0.0.0/8");
    assert!(n.contains(ip("10.0.0.1")));
    assert!(n.contains(ip("10.255.255.254")));
    assert!(!n.contains(ip("11.0.0.1")));

    let odd = net("192.168.4.0/22"); // 192.168.4.0 – 192.168.7.255
    assert!(odd.contains(ip("192.168.4.1")));
    assert!(odd.contains(ip("192.168.7.255")));
    assert!(!odd.contains(ip("192.168.8.0")));
    assert!(!odd.contains(ip("192.168.3.255")));

    let v6 = net("2001:db8::/32");
    assert!(v6.contains(ip("2001:db8::1")));
    assert!(!v6.contains(ip("2001:db9::1")));
}

/// `/0` trusts everything of that family — legal, and it must mean what it says rather than
/// accidentally matching nothing (or the other family).
#[test]
fn a_zero_prefix_trusts_the_whole_family_and_only_that_family() {
    let all_v4 = net("0.0.0.0/0");
    assert!(all_v4.contains(ip("1.2.3.4")));
    assert!(all_v4.contains(ip("203.0.113.9")));
    assert!(!all_v4.contains(ip("2001:db8::1")));
}

/// An IPv4-mapped IPv6 peer is the IPv4 client it denotes. A dual-stack listener produces
/// these, and without canonicalisation a correct `127.0.0.1` entry would match nothing —
/// failing closed, but silently, over a configuration that is right.
#[test]
fn ipv4_mapped_addresses_canonicalise_on_both_sides() {
    assert!(net("127.0.0.1").contains(ip("::ffff:127.0.0.1")));
    assert!(net("::ffff:127.0.0.1").contains(ip("127.0.0.1")));
    assert!(net("10.0.0.0/8").contains(ip("::ffff:10.1.2.3")));
    assert!(!net("10.0.0.0/8").contains(ip("::ffff:11.1.2.3")));
}

/// Families do not cross.
#[test]
fn mismatched_families_never_match() {
    assert!(!net("127.0.0.1").contains(ip("::1")));
    assert!(!net("::1").contains(ip("127.0.0.1")));
}

/// A CIDR with host bits set is refused rather than silently masked. `10.0.0.5/8` read as
/// `10.0.0.0/8` trusts a network the operator never typed — the widening this module exists to
/// prevent, arriving as a typo instead of as a decision.
#[test]
fn a_cidr_with_host_bits_set_is_refused_not_masked() {
    let err = ProxyNet::parse("10.0.0.5/8").expect_err("host bits set must not parse");
    assert!(err.contains("host bits"), "unhelpful reason: {err}");
    assert!(ProxyNet::parse("10.0.0.0/8").is_ok());
    // …and the same rule inside a byte.
    assert!(ProxyNet::parse("192.168.5.0/22").is_err());
    assert!(ProxyNet::parse("192.168.4.0/22").is_ok());
}

/// Every other way an entry can be wrong is an error with a reason, never a `ProxyNet`.
#[test]
fn malformed_entries_are_rejected() {
    for bad in [
        "",
        "  ",
        "not-an-ip",
        "10.0.0.0/",
        "10.0.0.0/33",
        "::/129",
        "10.0.0.0/eight",
        "10.0.0.0/8/8",
        "127.0.0.1:8080",
        "example.com",
    ] {
        assert!(
            ProxyNet::parse(bad).is_err(),
            "{bad:?} must not parse as a trusted proxy"
        );
    }
}

/// Surrounding whitespace is the `.env` copy-paste, not a different proxy.
#[test]
fn entries_tolerate_surrounding_whitespace() {
    assert_eq!(net("  127.0.0.1  "), net("127.0.0.1"));
    assert_eq!(net(" 10.0.0.0/8 "), net("10.0.0.0/8"));
}
