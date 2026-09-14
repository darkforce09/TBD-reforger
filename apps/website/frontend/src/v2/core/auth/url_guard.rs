//! The guard on every link the app renders: only a URL a browser follows, never one it runs.
//!
//! **Role:** answers whether a string may be used as a link target.
//! **Position:** the output side of the boundary. The backend validates what it stores; this
//! validates what is rendered, and the two fail independently.
//! **Signals & state:** none. A pure predicate over its argument.
//! **Invariants:** validating on the way in is not enough on its own. It governs only values that
//! arrived after it shipped, and only the writers that remember to call it — a backfill, an
//! operator's console, or a handler added later inherits nothing. Input validation says the value
//! was acceptable when it arrived; a renderer needs the stronger claim that it is safe now, in this
//! attribute, and only the place that renders it can make that claim.
//!
//! # Why the predicate exists twice
//!
//! The same rule lives in the API crate. This crate compiles to WebAssembly and cannot depend on a
//! server crate that links a database driver and an async runtime, so calling the shipped one is
//! not available. Two existing crates were considered as a shared home and both were rejected: the
//! map engine's compute core, because a browser-scheme allowlist filed under geometry and the
//! mission compiler is somewhere nobody auditing web security would look — and because four other
//! crates depend on it and would all inherit a URL-parsing dependency for a function about HTML
//! attributes; and a new crate existing only to hold a twelve-line predicate, which is a permanent
//! unit of ownership, review and build cost that this does not earn.
//!
//! So it is a port, and the duplication is paid for in the only currency that matters. Both copies
//! call the same compiled parser: the URL crate is already in this crate's dependency graph twice
//! over through the router and the server-function machinery, and the workspace lockfile unifies it
//! with the version the API builds against, so naming it directly adds nothing to the bundle and
//! makes the agreement structural rather than a coincidence maintained by hand. And the input table
//! lives once, in a file both crates include, so drift is a test failure in both crates on the same
//! commit.
//!
//! What is not claimed: that two copies are as good as one. They are not. This is the least-bad
//! arrangement available without inventing a crate, and the shared table is what makes it safe to
//! live with.

use url::Url;

/// Does `candidate` name an absolute `http` or `https` URL — one a browser will follow rather than
/// execute?
///
/// Byte for byte the same rule as the API crate's copy, deliberately. Every change here needs the
/// same change there, and the shared case table is what enforces it. The rule is:
///
/// * an allowlist of exactly two schemes, because a denylist enumerates an open set and loses to
///   the first spelling nobody wrote down;
/// * no ASCII control characters and no surrounding whitespace, because browsers strip leading
///   and trailing control characters and *delete* tab and newline from anywhere inside a link
///   target — so a string with a tab in the middle of the scheme renders as the scheme it was
///   spelling while satisfying any test applied to the raw text;
/// * a non-empty host, so that neither a bare scheme nor a scheme-relative target passes.
///
/// It is not an SSRF guard, carries no domain allowlist, and says nothing about the path, query or
/// fragment. An empty string is simply `false`; a caller for which empty means "no link" gets the
/// right answer anyway and should render its empty state.
///
/// # One measured caveat about the host clause
///
/// No test can currently tell whether the host is checked at all, and that is a property of the
/// parser rather than a gap anyone chose. Both allowed schemes are special ones, and for those the
/// parser refuses an empty host outright rather than returning a URL carrying one — so by the time
/// the host clause runs the host is already known non-empty, and deleting the clause leaves the
/// whole shared table green. It stays anyway: it is what holds if the allowlist is ever widened to
/// a scheme where an absent host is legitimate, or if the parser relaxes its rule. What must not
/// happen is reading a green suite as proof that this clause works — it is untested because it is
/// currently unreachable.
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
#[path = "tests/url_guard.rs"]
mod tests;
