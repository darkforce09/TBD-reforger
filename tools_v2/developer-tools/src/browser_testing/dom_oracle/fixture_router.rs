//! Resolution of one intercepted browser request into the reply the oracle serves for it.
//!
//! The oracle renders every route against the committed fixture corpus rather than a live backend,
//! so this table is the entire data surface a captured page can see. It is deliberately total: an
//! API call either has a fixture behind it, is one of the two protocol replies below, or is
//! reported as missing. Nothing is answered with a placeholder, because a page whose data silently
//! failed to arrive still renders a *stable* empty or error screen, and a stable screen is exactly
//! what the capture's settle loop accepts and `accept` would then write into a golden.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The fixture corpus — shared with the frontend's R-api round-trip tests and the editor smokes.
pub(super) fn fixtures_dir() -> PathBuf {
    crate::browser_testing::server::repo_root().join("apps/website/frontend/tests/fixtures/api")
}

/// An API request the fixture corpus does not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingFixture {
    /// The request as the page made it, query string included.
    pub url: String,
    /// The corpus file that would have answered it.
    pub expected_file: String,
}

/// What the router does with one intercepted request.
pub(super) enum Reply {
    /// Protocol plumbing the corpus deliberately does not own: the token endpoints have no
    /// rendered consumer, and pinning a token in a fixture would date the corpus, not describe it.
    Canned(Value),
    /// A fixture file, served with the media type its extension names.
    Fixture {
        path: PathBuf,
        content_type: &'static str,
    },
    /// Not an API call. The local static server answers it (the SPA bundle, fonts, map assets).
    Passthrough,
    /// An API call with nothing behind it.
    Missing(MissingFixture),
}

/// `/api/v1/<path>[?…]` + method → corpus file stem: `<METHOD>__` + the path with its trailing
/// slash stripped and every `/` replaced by `__`.
///
/// The method is part of the name because the corpus holds more than one verb for the same path;
/// a `GET`-only rule leaves a committed `POST__…` fixture unreachable and its route unfed.
fn stem_for(method: &str, url: &str) -> Option<String> {
    let idx = url.find("/api/v1/")?;
    let rest = &url[idx + "/api/v1/".len()..];
    let end = rest.find(['?', '#']).unwrap_or(rest.len());
    Some(format!(
        "{}__{}",
        method.to_ascii_uppercase(),
        rest[..end].trim_end_matches('/').replace('/', "__")
    ))
}

/// Corpus extensions, in resolution order, with the media type each one is served as.
///
/// `.sse.txt` exists because a Server-Sent Events body is delimited by a literal `\n\n` and is
/// therefore not expressible as JSON — see [`crate::browser_testing::cdp::Page::fulfill_raw`].
const EXTENSIONS: [(&str, &str); 2] = [
    ("json", "application/json"),
    ("sse.txt", "text/event-stream"),
];

/// The corpus file answering this request, if one is committed.
pub(super) fn fixture_for(method: &str, url: &str) -> Option<(PathBuf, &'static str)> {
    let stem = stem_for(method, url)?;
    let dir = fixtures_dir();
    EXTENSIONS.iter().find_map(|(ext, content_type)| {
        let path = dir.join(format!("{stem}.{ext}"));
        path.is_file().then_some((path, *content_type))
    })
}

/// Decide one request. `url` is the full request URL; `method` is its HTTP verb.
pub(super) fn route(method: &str, url: &str) -> Reply {
    if url.contains("/api/v1/auth/refresh") {
        return Reply::Canned(json!({
            "access_token": "acc-v",
            "refresh_token": "rt-v2",
            "expires_at": "2026-01-01T01:00:00Z"
        }));
    }
    if url.contains("/api/v1/auth/logout") {
        return Reply::Canned(json!({}));
    }
    if let Some((path, content_type)) = fixture_for(method, url) {
        return Reply::Fixture { path, content_type };
    }
    match stem_for(method, url) {
        Some(stem) => Reply::Missing(MissingFixture {
            url: url.to_string(),
            expected_file: format!("{stem}.json"),
        }),
        None => Reply::Passthrough,
    }
}

/// The capture failure raised when a route's data never arrived.
///
/// It names the corpus files rather than the symptom, because the symptom — an empty table, a
/// "Failed to load" line — is indistinguishable from a real UI state once it is serialized.
pub(super) fn missing_fixture_error(route_path: &str, missing: &[MissingFixture]) -> String {
    let mut lines = vec![format!(
        "{} request(s) at {route_path} have no fixture; capture refused \
         (an unfed page renders a stable error state, which is not a baseline):",
        missing.len()
    )];
    for m in missing {
        lines.push(format!(
            "  {} — add apps/website/frontend/tests/fixtures/api/{}",
            m.url, m.expected_file
        ));
    }
    lines.join("\n")
}

/// Read a fixture file as the bytes to serve. JSON is minified through `serde_json` so a
/// pretty-printed corpus file reaches the page as one compact body, exactly as the backend sends
/// it; every other media type is served byte for byte.
pub(super) fn body_bytes(path: &Path, content_type: &str) -> Option<Vec<u8>> {
    let raw = std::fs::read(path).ok()?;
    if content_type == "application/json" {
        let value: Value = serde_json::from_slice(&raw).ok()?;
        return serde_json::to_vec(&value).ok();
    }
    Some(raw)
}

#[cfg(test)]
#[path = "../tests/dom_oracle/fixture_router.rs"]
mod tests;
