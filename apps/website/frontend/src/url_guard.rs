//! The SPA's output-side URL guard. **T-405.**
//!
//! T-391 closed the write boundary: `handlers::telemetry::upsert_match` runs every incoming
//! `aar_replay_url` through `services::text::is_http_url` and answers 400 rather than storing a
//! `javascript:` URL. That guard survived 40 adversarial inputs and is not in question.
//!
//! It is also not sufficient, for two reasons that have nothing to do with how good it is:
//!
//!   1. **It only governs values that arrived after it shipped.** Rows written before it hold
//!      whatever they hold. (`migrations/0010_backfill_aar_replay_url_scheme.sql` cleans those
//!      out; this module is what stops the next one.)
//!   2. **It only governs the one writer that remembers to call it.** The database is not the
//!      only way a string reaches this SPA, and the next writer — a backfill script, an
//!      operator's `psql`, a handler somebody adds in six months — does not inherit the guard
//!      by being adjacent to it.
//!
//! Input validation says "this value was acceptable when it arrived". The renderer needs the
//! stronger claim: "this value is safe *now*, in this attribute". Only the sink can make that
//! claim, and only about itself. So the sink checks too. Neither half is redundant; they fail
//! independently.
//!
//! # Why this predicate exists twice
//!
//! `is_http_url` also lives at `apps/website/api/src/services/text.rs`, in the `website-api`
//! crate. This crate is `website-frontend`, compiled to wasm32 — it cannot depend on a server
//! crate that links sqlx, axum and tokio, so "just call the shipped one" is not on the table.
//!
//! **Two existing crates were considered as a shared home and both were rejected:**
//!
//!   * `crates/map-engine-core` — the only crate BOTH sides already depend on
//!     (`api/Cargo.toml` with `features = ["mission"]`, this crate natively with
//!     `default-features = false`). Technically available, and rejected on two counts. It is
//!     the map engine's compute core — geometry, the mission compiler, the CRDT document
//!     model — so a browser-scheme allowlist filed there is a thing no one auditing web
//!     security would think to look for. And it has four consumers beyond these two
//!     (`map-engine-render`, `map-engine-wasm`, `tools/tbd-tools`, `xtask`), all of which
//!     would inherit a `url`/`idna` dependency for a function about HTML attributes.
//!   * A NEW shared crate, existing only to hold one 12-line predicate. A crate is a permanent
//!     unit of ownership, review and build cost; this does not earn one. If a third or fourth
//!     genuinely-shared web utility ever appears, that is the moment to revisit — not before.
//!
//! So it is a port, and the duplication is paid for in the only currency that matters:
//!
//!   * **Same parser, same version, same bytes.** `url v2.5.8` is already in this crate's wasm
//!     dependency graph twice over — via `leptos_router`, and via `leptos → leptos_server →
//!     server_fn` — and the workspace lockfile unifies it with the version `website-api`
//!     builds against. Naming it a direct dependency adds nothing to the bundle and buys the
//!     guarantee that both implementations call *the same compiled `Url::parse`*. Agreement is
//!     structural, not a coincidence maintained by hand.
//!   * **Drift is a test failure, in both crates, on the same commit.** The input table lives
//!     once, at `apps/website/shared/is_http_url_cases.rs`, and both crates `include!` it. See
//!     that file; it is where new adversarial cases go.
//!
//! What is NOT claimed: that two copies are as good as one. They are not. This is the least-bad
//! arrangement available without inventing a crate, and the pin is what makes it safe to live
//! with.

use url::Url;

/// Does `candidate` name an absolute `http`/`https` URL — i.e. one a browser will *follow*
/// rather than *execute*?
///
/// **Byte-for-byte the same rule as `website_api::services::text::is_http_url`, deliberately.**
/// Every change here needs the same change there, and the shared table in
/// `apps/website/shared/is_http_url_cases.rs` is what enforces it. The comments on the original
/// carry the full rationale for each clause; the short version:
///
///   * an allowlist of exactly two schemes, because a `javascript:` denylist enumerates an open
///     set and loses to the first spelling nobody wrote down;
///   * no ASCII control characters and no leading/trailing whitespace, because browsers strip
///     leading/trailing C0-and-space and *delete* tab/CR/LF from anywhere inside an href — so
///     `"java\tscript:alert(1)"` renders as `javascript:alert(1)` while satisfying any test
///     applied to the raw string;
///   * a non-empty host, so neither a bare `https://` nor a scheme-relative `//evil.com` passes.
///
/// Same non-promises, too: not an SSRF guard, no domain allowlist, nothing about path/query/
/// fragment, and `""` is simply `false` (it is not a URL). A caller for which empty means "no
/// link" gets the right answer here anyway — `false` — and should render its empty state.
///
/// # One measured caveat about the host clause (T-405)
///
/// **No test can currently tell whether `host_str()` is checked at all**, and that is a property
/// of the parser rather than a gap anybody chose. `http` and `https` are WHATWG *special*
/// schemes, and for those the parser refuses an empty host outright rather than returning a `Url`
/// carrying one: `http:`, `http:/`, `http://`, `http:///`, `http://#f`, `http://?q`,
/// `http://:80/`, `http://:@/` and `https://:pass@/` all come back `Err(EmptyHost)`. So by the
/// time the host clause runs, the host is already known non-empty — deleting the clause leaves
/// the whole shared table green (measured, T-405).
///
/// It stays anyway, and is not dead weight: it is what holds if the allowlist is ever widened to
/// a non-special scheme (where `host_str()` legitimately *is* `None`), or if `url` ever relaxes
/// the special-scheme rule. What must not happen is reading a green suite as proof that this
/// clause works — it is untested because it is currently unreachable.
pub fn is_http_url(candidate: &str) -> bool {
    if candidate.chars().any(|c| c.is_ascii_control()) || candidate.trim() != candidate {
        return false;
    }
    match Url::parse(candidate) {
        Ok(u) => {
            matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty())
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shared input table — the same file `website-api`'s `services::text` tests include.
    // Editing either implementation without the other turns THIS red. See the file header.
    include!("../../shared/is_http_url_cases.rs");

    #[test]
    fn matches_the_api_guard_on_every_shared_case() {
        let mut wrong = Vec::new();
        for (input, expected) in IS_HTTP_URL_CASES {
            let got = is_http_url(input);
            if got != *expected {
                wrong.push(format!("  {input:?}: expected {expected}, got {got}"));
            }
        }
        assert!(
            wrong.is_empty(),
            "frontend is_http_url disagrees with the shared table on {} of {} cases \
             (the API guard is checked against the SAME table in \
             website-api services::text — if that one is green and this one is not, \
             the two implementations have DRIFTED):\n{}",
            wrong.len(),
            IS_HTTP_URL_CASES.len(),
            wrong.join("\n")
        );
    }

    /// The table is only worth anything if it actually contains the attacks. A future edit that
    /// trims it down to the easy cases would leave the test above green and meaningless.
    #[test]
    fn shared_table_still_carries_the_adversarial_inputs() {
        let inputs: Vec<&str> = IS_HTTP_URL_CASES.iter().map(|(i, _)| *i).collect();
        for required in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "java%73cript:alert(1)",
            "java\tscript:alert(1)",
            "java\nscript:alert(1)",
            "java\rscript:alert(1)",
            "jav\u{0}ascript:alert(1)",
            "\u{200b}javascript:alert(1)",
            "\u{feff}javascript:alert(1)",
            "\u{a0}javascript:alert(1)",
            "\u{ad}javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "vbscript:msgbox(1)",
            "blob:https://evil.com/1234",
            "file:///etc/passwd",
            "//evil.com",
            "http://",
        ] {
            assert!(
                inputs.contains(&required),
                "the shared case table no longer contains {required:?} — \
                 adversarial cases are not to be removed"
            );
        }
        let rejects = IS_HTTP_URL_CASES.iter().filter(|(_, ok)| !ok).count();
        assert!(
            rejects >= 40,
            "the shared table is down to {rejects} reject cases; T-391 threw 40 at this guard"
        );
    }
}
