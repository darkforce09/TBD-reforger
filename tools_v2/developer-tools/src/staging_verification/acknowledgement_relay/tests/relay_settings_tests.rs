//! The loopback rule on the listen address and the upstream origin, and the hold's margin.

use std::net::SocketAddr;
use std::path::PathBuf;

use super::{
    AGENT_REQUEST_TIMEOUT, DEFAULT_WITHHOLD, RelaySettings, UpstreamOrigin, loopback_listen_address,
};

#[test]
fn loopback_listen_addresses_are_accepted() {
    for listen in [
        "127.0.0.1:18085",
        "127.0.0.2:1",
        "[::1]:18085",
        "127.0.0.1:0",
    ] {
        let address = loopback_listen_address(listen).expect(listen);
        assert_eq!(address, listen.parse::<SocketAddr>().unwrap());
    }
}

#[test]
fn a_non_loopback_or_unnamed_listen_address_is_refused() {
    for listen in [
        "0.0.0.0:18085",
        "[::]:18085",
        "203.0.113.7:18085",
        "[2001:db8::1]:18085",
        "localhost:18085",
        "18085",
        "",
    ] {
        let refused = loopback_listen_address(listen).expect_err(listen);
        assert!(refused.to_string().starts_with("--listen"), "{refused}");
    }
}

#[test]
fn loopback_http_origins_are_accepted_and_localhost_is_pinned() {
    for (text, origin, host, address) in [
        (
            "http://127.0.0.1:8080",
            "http://127.0.0.1:8080",
            "127.0.0.1",
            "127.0.0.1:8080",
        ),
        (
            "http://127.0.0.1:8080/",
            "http://127.0.0.1:8080",
            "127.0.0.1",
            "127.0.0.1:8080",
        ),
        (
            "http://localhost:8080",
            "http://localhost:8080",
            "localhost",
            "127.0.0.1:8080",
        ),
        (
            "http://LOCALHOST:8080",
            "http://localhost:8080",
            "localhost",
            "127.0.0.1:8080",
        ),
        (
            "http://[::1]:8080",
            "http://[::1]:8080",
            "[::1]",
            "[::1]:8080",
        ),
        (
            "http://127.0.0.9",
            "http://127.0.0.9:80",
            "127.0.0.9",
            "127.0.0.9:80",
        ),
    ] {
        let upstream = UpstreamOrigin::parse(text).expect(text);
        assert_eq!(upstream.as_str(), origin, "{text}");
        assert_eq!(upstream.host(), host, "{text}");
        assert_eq!(
            upstream.address(),
            address.parse::<SocketAddr>().unwrap(),
            "{text}"
        );
    }
}

#[test]
fn an_upstream_off_loopback_or_beyond_a_bare_http_origin_is_refused() {
    for text in [
        "https://127.0.0.1:8080",
        "http://203.0.113.7:8080",
        "http://0.0.0.0:8080",
        "http://[2001:db8::1]:8080",
        "http://example.org:8080",
        "http://localhost.example.org:8080",
        "http://operator:secret@127.0.0.1:8080",
        "http://127.0.0.1:8080/api",
        "http://127.0.0.1:8080/?next=1",
        "http://127.0.0.1:8080/#top",
        "127.0.0.1:8080",
    ] {
        let refused = UpstreamOrigin::parse(text).expect_err(text);
        assert!(refused.to_string().starts_with("--upstream"), "{refused}");
    }
}

#[test]
fn a_request_target_joins_the_origin_verbatim() {
    let upstream = UpstreamOrigin::parse("http://127.0.0.1:8080").unwrap();
    let target = upstream
        .target("/api/v1/fleet-executor/commands/claim?probe=a%20b")
        .unwrap();
    assert_eq!(
        target.as_str(),
        "http://127.0.0.1:8080/api/v1/fleet-executor/commands/claim?probe=a%20b"
    );
    assert!(upstream.target("api/v1/relative").is_err());
}

#[test]
fn the_flags_build_settings_whose_hold_outlasts_the_agent_timeout() {
    let settings = RelaySettings::from_flags(
        "127.0.0.1:18085",
        "http://127.0.0.1:8080",
        PathBuf::from("/run/user/1000/acknowledgement-dropping-relay-5/control.sock"),
    )
    .unwrap();
    assert_eq!(settings.withhold, DEFAULT_WITHHOLD);
    assert!(DEFAULT_WITHHOLD >= AGENT_REQUEST_TIMEOUT + std::time::Duration::from_secs(5));
}
