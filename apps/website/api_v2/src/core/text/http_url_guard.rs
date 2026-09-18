//! The crate's URL write-boundary guard: does a candidate string name an absolute `http` /
//! `https` URL?

use url::Url;

/// Does `candidate` name an absolute `http`/`https` URL — i.e. one that a browser will *follow*
/// rather than *execute* when it lands in an `<a href>` or an `<img src>`?
///
/// Callers reject with 400 and do not store. Storing the value and escaping it on the way out is
/// the wrong trade: HTML-escaping does nothing to a `javascript:` scheme in an `href` (it is not
/// a quote-breakout, it is a perfectly well-formed attribute whose *content* executes on click),
/// and even where escaping would help, it leaves a live payload in the database for every other
/// reader — a CSV export, a Discord webhook, a page nobody has written yet — each of which has to
/// remember independently, forever. Rejecting at the write is the only version that stays fixed.
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
///   before calling — see `match_telemetry::handlers::match_upsert::upsert_match`.
///
/// # The sinks that use it
///
/// Every URL column writes through this guard at its boundary:
/// `announcements.thumbnail_url` (`community_content/handlers/announcements_admin.rs`), `events.banner_image_url`
/// (`operations/handlers/event_create_update.rs`), `missions.thumbnail_url` (`missions/handlers/mission_lifecycle.rs`),
/// `users.avatar_url` (`handlers/auth/oauth.rs`, which `format!`-builds a CDN URL out of an
/// unvalidated Discord avatar hash), and `matches.replay_url`
/// (`match_telemetry::handlers::match_upsert::upsert_match`, the worked example).
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
#[path = "tests/http_url_guard.rs"]
mod tests;
