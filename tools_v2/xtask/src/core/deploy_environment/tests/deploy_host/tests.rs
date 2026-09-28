use super::*;
use std::net::{Ipv6Addr, SocketAddrV4, SocketAddrV6};

#[test]
fn a_host_splits_into_its_user_and_name() {
    let with_user = DeployHost::parse("deploy@192.0.2.10").expect("parses");
    assert_eq!(with_user.user(), Some("deploy"));
    assert_eq!(with_user.host(), "192.0.2.10");
    assert_eq!(with_user.ssh_destination(), "deploy@192.0.2.10");
    assert_eq!(with_user.home_directory().as_deref(), Some("/home/deploy"));
    assert_eq!(with_user.tbd_folder().as_deref(), Some("/home/deploy/tbd"));

    let bare = DeployHost::parse("staging.example").expect("parses");
    assert_eq!(bare.user(), None);
    assert_eq!(bare.ssh_destination(), "staging.example");
    assert_eq!(bare.home_directory(), None);
    assert_eq!(bare.tbd_folder(), None);
}

#[test]
fn malformed_hosts_are_refused() {
    for (value, problem) in [
        ("", "names no host"),
        ("deploy@", "names no host"),
        ("@192.0.2.10", "names no user"),
        ("a@b@c", "second `@`"),
        ("deploy@ 192.0.2.10", "whitespace"),
        ("-oProxyCommand=x", "starts with `-`"),
        ("deploy@-oProxyCommand=x", "starting with `-`"),
    ] {
        let error = DeployHost::parse(value).expect_err(value);
        assert!(error.contains(problem), "{value:?}: {error}");
    }
}

#[test]
fn the_first_ipv4_address_is_picked_after_ipv6_ones() {
    let v6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0));
    let v4 = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0));
    assert_eq!(first_ipv4_of([v6, v4]), Some(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(first_ipv4_of([v6]), None);
    assert_eq!(first_ipv4_of([]), None);
}

#[test]
fn an_ipv4_literal_is_returned_without_a_lookup() {
    assert_eq!(
        first_ipv4_address("192.0.2.10"),
        Ok(Ipv4Addr::new(192, 0, 2, 10))
    );
    let host = DeployHost::parse("deploy@192.0.2.10").expect("parses");
    assert_eq!(host.resolve_ipv4(), Ok(Ipv4Addr::new(192, 0, 2, 10)));
}

/// `localhost` comes from `/etc/hosts`, so this needs no network; resolvers that list `::1` first
/// are the case the IPv4 pick exists for.
#[test]
fn localhost_resolves_to_its_ipv4_loopback() {
    assert_eq!(first_ipv4_address("localhost"), Ok(Ipv4Addr::LOCALHOST));
}

#[test]
fn an_ipv6_only_name_has_no_ipv4_address() {
    let error = first_ipv4_address("::1").expect_err("IPv6 literal");
    assert!(error.contains("IPv6 only: ::1"), "{error}");
}
