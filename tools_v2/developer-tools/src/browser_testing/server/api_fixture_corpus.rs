//! The recorded API corpus route of the gate server.
//!
//! **Role:** answers `/api/…` requests from a directory of recorded response bodies, so a gate
//! can serve the app's public reads (the ballistics catalogs) from a real HTTP origin that a
//! service worker caches exactly like the API.
//! **Position:** consulted by the gate server's handler (`super`) before the API proxy when
//! [`super::ServeConfig::api_fixture_corpus`] is set; the directory is the frontend's recorded
//! API corpus (`apps/website/frontend/tests/fixtures/api`), whose file names are
//! `<METHOD>__<path after /api/v1/ with every / as __>.json`.
//! **Signals & state:** a process-wide set of corpus directories whose API is down
//! ([`set_api_down`]), which a gate flips to act out a proxy (Caddy in production) whose API
//! upstream has stopped; each request reads one file.
//! **Invariants:** while its corpus is down every `/api/` request answers `502 Bad Gateway` with
//! an empty body, exactly as a proxy does with no upstream; otherwise only a `GET` whose
//! `GET__….json` file exists answers 200, with the file's bytes unchanged, and every other
//! `/api/` request answers 404 with a JSON error naming the file looked for, never the
//! single-page fallback; a file name holds only ASCII letters, digits, `-`, `_` and `.`, so no
//! request reaches outside the corpus directory; the down set is keyed by directory, so servers
//! over other corpora are never affected.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use axum::http::{Method, StatusCode};
use axum::response::Response;

use super::respond;

/// The API prefix the recorded file names are relative to.
const API_PREFIX: &str = "/api/v1/";

/// Corpus directories whose `/api/` route answers as a proxy whose API is down.
static API_DOWN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Marks the API behind `corpus` down (every `/api/` request answers `502`) or up again.
pub fn set_api_down(corpus: &Path, down: bool) {
    let mut set = API_DOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    set.retain(|dir| dir != corpus);
    if down {
        set.push(corpus.to_path_buf());
    }
}

/// Whether the API behind `corpus` is marked down.
#[must_use]
pub fn is_api_down(corpus: &Path) -> bool {
    API_DOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .any(|dir| dir == corpus)
}

/// The corpus file name answering `method path` (`path` without its query), or `None` when the
/// request is outside `/api/v1/` or its name would hold a character outside the safe set.
#[must_use]
pub fn corpus_file_name(method: &Method, path: &str) -> Option<String> {
    let rest = path.strip_prefix(API_PREFIX)?.trim_end_matches('/');
    if rest.is_empty() {
        return None;
    }
    let name = format!(
        "{}__{}.json",
        method.as_str().to_ascii_uppercase(),
        rest.replace('/', "__")
    );
    let safe = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    (safe && !name.contains("..")).then_some(name)
}

/// Answers one `/api/` request from `corpus`.
pub(super) async fn answer(corpus: &Path, method: &Method, path: &str) -> Response {
    if is_api_down(corpus) {
        return respond(StatusCode::BAD_GATEWAY, None, Vec::new());
    }
    let name = corpus_file_name(method, path);
    if method == Method::GET
        && let Some(name) = &name
        && let Ok(body) = tokio::fs::read(corpus.join(name)).await
    {
        return respond(StatusCode::OK, Some("application/json"), body);
    }
    let looked_for = name.unwrap_or_else(|| "(no corpus name)".to_string());
    let body = serde_json::json!({
        "error": {
            "code": "not_found",
            "message": format!("no recorded response for {method} {path}: {looked_for}"),
        }
    });
    respond(
        StatusCode::NOT_FOUND,
        Some("application/json"),
        body.to_string().into_bytes(),
    )
}

#[cfg(test)]
#[path = "../tests/server/api_fixture_corpus.rs"]
mod tests;
