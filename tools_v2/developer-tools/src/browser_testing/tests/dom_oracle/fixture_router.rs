use super::*;

const BASE: &str = "http://localhost:5196";

fn stem(method: &str, path: &str) -> String {
    match route(method, &format!("{BASE}{path}")) {
        Reply::Missing(m) => m.expected_file,
        Reply::Fixture { path, .. } => path.file_name().unwrap().to_string_lossy().into_owned(),
        Reply::Canned(_) => "<canned>".into(),
        Reply::Passthrough => "<passthrough>".into(),
    }
}

#[test]
fn a_path_becomes_a_method_prefixed_corpus_name() {
    assert_eq!(
        stem("GET", "/api/v1/me/link/status"),
        "GET__me__link__status.json"
    );
}

/// The committed `POST__fire-missions__solve.json` answers the mortar page. A `GET`-only naming
/// rule looked past it, so the route rendered without its firing solution.
#[test]
fn the_method_selects_the_fixture_so_a_post_corpus_entry_is_reachable() {
    assert_eq!(
        stem("POST", "/api/v1/fire-missions/solve"),
        "POST__fire-missions__solve.json"
    );
    // The same path under a different verb is a different fixture, and its absence is reported.
    assert_eq!(
        stem("DELETE", "/api/v1/fire-missions/solve"),
        "DELETE__fire-missions__solve.json"
    );
}

#[test]
fn a_query_string_does_not_change_which_fixture_answers() {
    assert_eq!(
        stem("GET", "/api/v1/cms/announcements?limit=100"),
        stem("GET", "/api/v1/cms/announcements")
    );
    assert_eq!(
        stem("GET", "/api/v1/missions/"),
        stem("GET", "/api/v1/missions")
    );
}

/// The status stream is Server-Sent Events, so its fixture carries the wire bytes rather than a
/// JSON document, and must be served as `text/event-stream` or the frame splitter sees no frame.
#[test]
fn the_status_stream_resolves_to_an_event_stream_fixture() {
    let url = format!("{BASE}/api/v1/servers/00000000-0000-4000-d000-000000000001/status/stream");
    match route("GET", &url) {
        Reply::Fixture { path, content_type } => {
            assert_eq!(content_type, "text/event-stream");
            assert!(
                path.to_string_lossy().ends_with(
                    "GET__servers__00000000-0000-4000-d000-000000000001__status__stream.sse.txt"
                ),
                "resolved {}",
                path.display()
            );
        }
        _ => panic!("the status stream must resolve to a committed SSE fixture"),
    }
}

/// The frame delimiter has to survive on the way to the page. Routed through `serde_json` it comes
/// out as the two characters `\n` — see `cdp::Page::fulfill_raw`.
#[test]
fn an_event_stream_body_keeps_its_literal_frame_delimiter() {
    let url = format!("{BASE}/api/v1/servers/00000000-0000-4000-d000-000000000001/status/stream");
    let Reply::Fixture { path, content_type } = route("GET", &url) else {
        panic!("the status stream must resolve to a committed SSE fixture");
    };
    let bytes = body_bytes(&path, content_type).expect("the SSE fixture is readable");
    assert!(
        bytes.windows(2).any(|w| w == b"\n\n"),
        "an SSE fixture without a literal blank line delivers no frame"
    );
    assert!(bytes.starts_with(b"data: "));
}

/// JSON is minified on the way out so a pretty-printed corpus file still reaches the page as the
/// compact body the backend sends.
#[test]
fn a_json_fixture_is_served_minified() {
    let Reply::Fixture { path, content_type } = route("GET", &format!("{BASE}/api/v1/me")) else {
        panic!("GET__me.json is committed");
    };
    let bytes = body_bytes(&path, content_type).expect("GET__me.json is readable");
    assert!(!bytes.contains(&b'\n'), "the served body must be one line");
}

/// The hole this closes: an API call with no fixture used to be answered `{}` + 200, so the page
/// rendered a stable error state that the settle loop accepted as a baseline.
#[test]
fn an_unanswered_api_call_is_reported_rather_than_filled_in() {
    match route("GET", &format!("{BASE}/api/v1/there-is-no-such-resource")) {
        Reply::Missing(m) => {
            assert_eq!(m.expected_file, "GET__there-is-no-such-resource.json");
            assert!(m.url.ends_with("/api/v1/there-is-no-such-resource"));
        }
        _ => panic!("an unanswered API call must be reported"),
    }
}

/// Everything that is not an API call still reaches the local static server — the SPA bundle, its
/// wasm, fonts and map assets are served, not reported.
#[test]
fn a_non_api_request_passes_through_to_the_static_server() {
    assert!(matches!(
        route("GET", &format!("{BASE}/index.html")),
        Reply::Passthrough
    ));
    assert!(matches!(
        route("GET", &format!("{BASE}/map-assets/everon/terrain.tbdd")),
        Reply::Passthrough
    ));
}

/// The token endpoints have no rendered consumer; pinning a token in the corpus would date it.
#[test]
fn the_token_endpoints_are_answered_without_a_fixture() {
    assert!(matches!(
        route("POST", &format!("{BASE}/api/v1/auth/refresh")),
        Reply::Canned(_)
    ));
    assert!(matches!(
        route("POST", &format!("{BASE}/api/v1/auth/logout")),
        Reply::Canned(_)
    ));
}

#[test]
fn the_refusal_names_every_missing_file_and_the_url_that_wanted_it() {
    let msg = missing_fixture_error(
        "/deployments",
        &[
            MissingFixture {
                url: "http://localhost:5196/api/v1/me/leave-requests".into(),
                expected_file: "GET__me__leave-requests.json".into(),
            },
            MissingFixture {
                url: "http://localhost:5196/api/v1/admin/leave-requests".into(),
                expected_file: "GET__admin__leave-requests.json".into(),
            },
        ],
    );
    assert!(msg.contains("2 request(s) at /deployments"));
    assert!(msg.contains("GET__me__leave-requests.json"));
    assert!(msg.contains("GET__admin__leave-requests.json"));
    assert!(msg.contains("/api/v1/admin/leave-requests"));
}
