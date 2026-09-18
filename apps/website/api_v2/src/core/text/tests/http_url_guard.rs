//! Unit coverage for the URL write-boundary guard, plus the shared case table that keeps this
//! implementation and the SPA's copy from drifting apart.

use super::*;

/// Every spelling of "execute this instead of fetching it" the guard has to survive.
///
/// Grouped rather than split into one test per case on purpose: the value of this list is
/// that it is a *list* — the next person adding a scheme trick should add a line here, and
/// a failure names the exact input.
#[test]
fn rejects_script_schemes_in_every_spelling() {
    for bad in [
        // The defect this guard closes: this is what executed from `<a href>` on click.
        "javascript:alert(1)",
        // Case — the reason this is an allowlist and not a `starts_with("javascript:")`.
        "JaVaScRiPt:alert(1)",
        "JAVASCRIPT:alert(1)",
        // Leading/trailing whitespace: a browser strips it before parsing, so a raw-string
        // check sees "not a scheme" while the browser sees one.
        " javascript:alert(1)",
        "\tjavascript:alert(1)",
        "\njavascript:alert(1)",
        "\r\n javascript:alert(1)",
        "javascript:alert(1) ",
        // Control characters *inside* the scheme: browsers delete tab/CR/LF anywhere in a
        // URL, so all three of these resolve to `javascript:alert(1)`.
        "java\tscript:alert(1)",
        "java\nscript:alert(1)",
        "java\rscript:alert(1)",
        "jav\u{0}ascript:alert(1)",
        "\u{0}javascript:alert(1)",
        // Other executing / content-bearing schemes.
        "data:text/html,<script>alert(1)</script>",
        "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
        "vbscript:msgbox(1)",
        "VBScript:msgbox(1)",
        "file:///etc/passwd",
        "blob:https://evil.com/1234",
        // Scheme-relative and relative: no scheme at all, so nothing to allow.
        "//evil.com",
        "//evil.com/replay.json",
        "\\\\evil.com\\share",
        "/replays/local.json",
        "replay.json",
        // Well-formed-looking but hostless.
        "http://",
        "https://",
        "",
        "   ",
    ] {
        assert!(!is_http_url(bad), "guard accepted {bad:?}");
    }
}

/// The other half, and the half that keeps the guard alive: a guard that 400s a real replay
/// link gets reverted by whoever ships next, and the hole comes back.
#[test]
fn accepts_ordinary_http_and_https_urls() {
    for good in [
        "http://example.com",
        "http://example.com/",
        "https://aar.tbd/replays/abc.json",
        // Query string.
        "https://aar.tbd/replays?match=abc-123&format=json",
        // Port.
        "https://aar.tbd:8443/replays/abc.json",
        "http://192.168.1.10:8080/replay",
        // Fragment.
        "https://aar.tbd/replays/abc.json#t=30",
        // All three at once, which is what a real replay link looks like.
        "https://aar.tbd:8443/replays/abc.json?token=xyz&v=2#t=30",
        // Percent-encoding and sub-delimiters in the path survive untouched.
        "https://aar.tbd/replays/Operation%20Red%20Dawn.json",
        "https://aar.tbd/a/b/c/d/e?x=1;y=2",
        // Userinfo and IDN are legal URLs; the guard is about the scheme, not taste.
        "https://user:pass@aar.tbd/replay",
        "https://xn--n3h.example/replay",
        // Uppercase scheme is a legitimate URL (the parser lowercases it for us).
        "HTTPS://AAR.TBD/replays/ABC.json",
    ] {
        assert!(is_http_url(good), "guard rejected {good:?}");
    }
}

/// The documented non-promises, pinned so nobody reads the allowlist as "safe to fetch".
#[test]
fn does_not_pretend_to_be_an_ssrf_guard() {
    assert!(is_http_url("http://127.0.0.1/"));
    assert!(is_http_url("http://localhost:8080/admin"));
    assert!(is_http_url("http://169.254.169.254/latest/meta-data/"));
    assert!(is_http_url("http://[::1]/"));
}

// ── The anti-drift pin ───────────────────────────────────────────────────────────────────
//
// The SPA cannot call this function — `website-frontend` compiles to wasm32 and cannot link a
// crate that pulls sqlx/axum/tokio — so the predicate is ported to
// `apps/website/frontend/src/url_guard.rs` and guards the render sink there. Two copies of a
// security predicate are only tolerable if they cannot drift apart quietly.
//
// This is the mechanism that stops them. The input table is a single file, `include!`d by
// BOTH crates, and each runs its own implementation over it. Change either implementation
// without the other and the OTHER crate's suite goes red on the same commit. New adversarial
// cases go in that file rather than here, so both sides get them at once.
include!("../../../../../shared/is_http_url_cases.rs");

#[test]
fn matches_the_frontend_guard_on_every_shared_case() {
    let mut wrong = Vec::new();
    for (input, expected) in IS_HTTP_URL_CASES {
        let got = is_http_url(input);
        if got != *expected {
            wrong.push(format!("  {input:?}: expected {expected}, got {got}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "api is_http_url disagrees with the shared table on {} of {} cases (the frontend \
         guard runs against the SAME table in website-frontend `url_guard` — if that one is \
         green and this one is not, the two implementations have DRIFTED):\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
}
