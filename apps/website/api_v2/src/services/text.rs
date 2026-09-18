//! Text utilities — Rust port of `services/text.go`. Snippet/truncate, an HTML cleaner kept for
//! callers that truly render HTML, plus the crate's one URL write-boundary guard
//! ([`is_http_url`], T-391).
//!
//! **T-239 contract (announcement bodies):** the SPA renders `announcements.body` as a Leptos
//! **text** node (`announcements.rs` `{p.to_string()}`), not via `inner_html`. There is no live
//! HTML sink for UGC. Applying [`sanitize_html`] (ammonia) before persist therefore buys zero
//! XSS defence and **double-escapes** bare `<` / `&` (ammonia escapes, then Leptos escapes again
//! → authors see literal `a &lt; b`). CMS stores body as authored plain text; Leptos owns the
//! single escape at render. The old open item ("exact UGCPolicy→ammonia allowlist + golden +
//! no-XSS property test") is **closed** — without an HTML sink that programme is defence theatre,
//! not a live-bug fix. Keep [`sanitize_html`] only for a future *real* HTML field.

use std::sync::OnceLock;

use url::Url;

/// HTML cleaner (ammonia). **Not** the announcement-body path — see module docs / T-239.
///
/// Go used `bluemonday.UGCPolicy()`; ammonia is a bounded engine deviation (gate G8). Do **not**
/// call this on a field that the SPA will render as a text node.
pub fn sanitize_html(body: &str) -> String {
    static CLEANER: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    CLEANER
        .get_or_init(ammonia::Builder::default)
        .clean(body)
        .to_string()
}

/// Short plain-ish preview: collapse whitespace then [`truncate`] to `n` runes.
pub fn snippet(body: &str, n: usize) -> String {
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&collapsed, n)
}

/// Shorten to at most `n` runes, appending `…` when cut (may exceed `n` by one rune).
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let head: String = s.chars().take(n).collect();
    format!("{head}…")
}

/// Shorten so the result — ellipsis included — never exceeds `n` runes (hard caps).
pub fn cap_runes(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let head: String = s.chars().take(n - 1).collect();
    format!("{head}…")
}

/// Does `candidate` name an absolute `http`/`https` URL — i.e. one that a browser will *follow*
/// rather than *execute* when it lands in an `<a href>` or an `<img src>`?
///
/// This is the crate's URL write-boundary guard. **T-391.** Callers reject with 400 and do not
/// store. Storing the value and escaping it on the way out is the wrong trade: HTML-escaping
/// does nothing to a `javascript:` scheme in an `href` (it is not a quote-breakout, it is a
/// perfectly well-formed attribute whose *content* executes on click), and even where escaping
/// would help, it leaves a live payload in the database for every other reader — a CSV export,
/// a Discord webhook, a page nobody has written yet — each of which has to remember
/// independently, forever. Rejecting at the write is the only version that stays fixed.
///
/// # The rule is an allowlist of two
///
/// `http` and `https` pass. Everything else fails. Deliberately **not** a `javascript:`
/// denylist: a denylist enumerates an open set and loses to the first spelling nobody wrote
/// down — `JaVaScRiPt:`, `data:text/html,…`, `vbscript:`, `jav\tascript:`, `\0javascript:`,
/// and whatever the next browser ships. Two accepted schemes fail closed.
///
/// # What it promises
///
/// - The scheme is `http` or `https`, read **after** the WHATWG parser has lowercased it, so
///   `JaVaScRiPt:` is the same string to this function as `javascript:`.
/// - The URL is absolute and has a **non-empty host**, so neither a bare `https://` nor a
///   scheme-relative `//evil.com` slips through on "well, it isn't `javascript:`".
/// - The value carries **no ASCII control character and no leading or trailing whitespace**, so
///   the bytes checked here are the bytes a browser will parse. This is the load-bearing half
///   and the reason a `starts_with("http")` check is not enough: browsers strip leading and
///   trailing C0-and-space from a URL attribute and *delete* tab, CR and LF from anywhere
///   inside it, so `"\tjava\nscript:alert(1)"` resolves as `javascript:alert(1)` while
///   satisfying any test applied to the raw string. Refusing those characters outright means
///   the stored form and the resolved form cannot disagree.
///
/// # What it does NOT promise
///
/// - **Not an SSRF guard.** `http://127.0.0.1/`, `http://localhost/` and
///   `http://169.254.169.254/latest/meta-data/` all pass, because they are all real `http`
///   URLs. Any caller that *fetches* a stored URL instead of handing it to a browser needs its
///   own host/network check; this function will not supply one.
/// - **No domain allowlist, no reachability check, no content check.** A value that passes is
///   well-formed, not trustworthy, and not known to be a replay.
/// - **Nothing about the path, query or fragment.** They are free text by design; only the
///   scheme can execute, and only the scheme is checked.
/// - **Not an output encoder.** A consumer with its own escaping problem still has it — a CSV
///   export must still escape for CSV, a Discord message for Markdown. Passing this guard is
///   not permission to interpolate the value anywhere.
/// - **Nothing about `""`.** An empty string is not a URL and carries no scheme, so it is
///   simply `false` here. A caller for which empty means "no link" must test for that itself,
///   before calling — see `handlers::telemetry::upsert_match`.
///
/// # Adopting it on the other sinks
///
/// The remaining URL columns share this same absent guard and live in handlers T-391 does not
/// own: `announcements.thumbnail_url` (`handlers/cms.rs:164`, `:260`),
/// `events.banner_image_url` (`handlers/events.rs:624`, `:1215`), `missions.thumbnail_url`
/// (`handlers/missions.rs:396`, `:509`), and `users.avatar_url` (`handlers/oauth.rs:119`, which
/// is public-tier and `format!`-builds a CDN URL out of an unvalidated Discord avatar hash).
/// Each is one `if !is_http_url(v) { return Err(ApiError::bad_request(..)) }` at its write
/// boundary; `handlers::telemetry::upsert_match` is the worked example.
pub fn is_http_url(candidate: &str) -> bool {
    // Both checks run *before* the parser, because their entire purpose is to make the parse
    // agree with the stored bytes. `is_ascii_control` covers NUL, tab, CR and LF in one rule;
    // `trim` covers the leading/trailing space the parser would silently discard.
    if candidate.chars().any(|c| c.is_ascii_control()) || candidate.trim() != candidate {
        return false;
    }
    match Url::parse(candidate) {
        // `scheme()` comes back lowercased, and a scheme-relative or otherwise base-less input
        // (`//evil.com`, `/replays/x`, `x.json`) never reaches this arm — it fails to parse.
        // `host_str()` is `None` for the non-special schemes that do parse, which is why the
        // host check is expressed as "must be present and non-empty" rather than as an unwrap.
        Ok(u) => {
            matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty())
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every spelling of "execute this instead of fetching it" the guard has to survive.
    ///
    /// Grouped rather than split into one test per case on purpose: the value of this list is
    /// that it is a *list* — the next person adding a scheme trick should add a line here, and
    /// a failure names the exact input.
    #[test]
    fn rejects_script_schemes_in_every_spelling() {
        for bad in [
            // The literal defect: this is what executed from `<a href>` on click.
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

    // ── T-405: the anti-drift pin ────────────────────────────────────────────────────────────
    //
    // The SPA cannot call this function — `website-frontend` compiles to wasm32 and cannot link a
    // crate that pulls sqlx/axum/tokio — so T-405 ported the predicate to
    // `apps/website/frontend/src/url_guard.rs` and guarded the render sink with it. Two copies of
    // a security predicate are only tolerable if they cannot drift apart quietly.
    //
    // This is the mechanism that stops them. The input table is a single file, `include!`d by
    // BOTH crates, and each runs its own implementation over it. Change either implementation
    // without the other and the OTHER crate's suite goes red on the same commit. New adversarial
    // cases go in that file rather than here, so both sides get them at once.
    include!("../../../shared/is_http_url_cases.rs");

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

    #[test]
    fn snippet_collapses_and_truncates() {
        assert_eq!(snippet("  hello   world\n\tfoo ", 100), "hello world foo");
        assert_eq!(snippet("aaaaaa", 3), "aaa…");
    }

    #[test]
    fn cap_runes_respects_hard_cap() {
        assert_eq!(cap_runes("hello", 10), "hello");
        assert_eq!(cap_runes("hello", 3).chars().count(), 3); // "he…"
    }

    #[test]
    fn sanitize_strips_scripts() {
        let out = sanitize_html("<p>ok</p><script>alert(1)</script>");
        assert!(out.contains("ok"));
        assert!(!out.contains("<script"));
    }

    /// Golden: ammonia HTML-escapes bare `<` / `&`. This is exactly why CMS must not call
    /// [`sanitize_html`] on announcement bodies that Leptos then text-escapes (T-239).
    ///
    /// RED perturbation: flip the `assert_ne!` to `assert_eq!` (claim identity) — this test
    /// goes red, proving the pin still observes ammonia's mutation rather than a no-op cleaner.
    #[test]
    fn sanitize_html_escapes_bare_angle_brackets_and_ampersands() {
        let authored = "Damage threshold: a < b & c > d";
        let cleaned = sanitize_html(authored);
        assert_ne!(
            cleaned, authored,
            "ammonia must mutate plain text containing < / & (else the T-239 double-escape \
             diagnosis is stale)"
        );
        assert!(
            cleaned.contains("&lt;") || cleaned.contains("&amp;"),
            "expected HTML entities in {cleaned:?}"
        );
        // Scripts still die — the cleaner is real HTML sanitation, just the wrong tool for
        // a text-rendered field.
        assert!(!sanitize_html("<script>alert(1)</script>").contains("<script"));
    }

    /// Round-trip pin for the text-field contract: snippet derivation must not introduce
    /// HTML entities. RED: replace `snippet` with `sanitize_html` below — fails on `a < b`.
    #[test]
    fn snippet_preserves_bare_angle_brackets() {
        let authored = "a < b & c";
        let snip = snippet(authored, 200);
        assert_eq!(snip, authored);
        assert!(!snip.contains("&lt;"));
        assert!(!snip.contains("&amp;"));
    }
}
