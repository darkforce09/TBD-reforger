use axum::http::{HeaderMap, HeaderValue, header};

use super::observability_bearer_matches;
use api_configuration::configuration::Config;

fn config(token: &str) -> Config {
    let mut cfg = Config::for_tests("postgres://unused", "unit-test-jwt-secret-long-enough");
    cfg.observability_token = token.to_owned();
    cfg
}

fn bearer(value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
    headers
}

#[test]
fn the_configured_bearer_matches() {
    assert!(observability_bearer_matches(
        &config("scrape-secret"),
        &bearer("Bearer scrape-secret")
    ));
}

#[test]
fn a_wrong_missing_or_unprefixed_token_does_not_match() {
    let cfg = config("scrape-secret");
    assert!(!observability_bearer_matches(
        &cfg,
        &bearer("Bearer scrape-secreT")
    ));
    assert!(!observability_bearer_matches(
        &cfg,
        &bearer("scrape-secret")
    ));
    assert!(!observability_bearer_matches(&cfg, &HeaderMap::new()));
}

#[test]
fn an_unset_token_matches_nothing_not_even_an_empty_bearer() {
    let cfg = config("");
    assert!(!observability_bearer_matches(&cfg, &bearer("Bearer ")));
    assert!(!observability_bearer_matches(&cfg, &HeaderMap::new()));
}

#[test]
fn the_retired_service_token_header_grants_nothing() {
    let cfg = config("scrape-secret");
    let mut headers = HeaderMap::new();
    headers.insert("x-service-token", HeaderValue::from_static("scrape-secret"));
    assert!(!observability_bearer_matches(&cfg, &headers));
}
