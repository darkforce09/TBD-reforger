//! The content URL policy in the app: which link targets and image sources authored content may
//! carry into an `href` or a `src`.
//!
//! **Role:** answers a safe link target ([`safe_link_href`]) or a safe image source
//! ([`safe_image_src`]) for authored content, and whether a safe link leaves the site
//! ([`is_external_link`]).
//! **Position:** a render-time check for any page that writes authored content into an attribute
//! (the doctrine wiki's links and images, the vehicle database's profile images). The backend
//! applies the same policy when it saves and parses that content
//! (`crates/api/api_foundation/src/text/content_url_policy.rs`), so this is the second check, made
//! where the attribute is written.
//! **Signals & state:** none; pure functions over the candidate string.
//! **Invariants:**
//! - The policy is an allowlist of prefixes, never a denylist of bad schemes: an image is
//!   `https://…` or a site path `/…`; a link is additionally `http://…`, `mailto:…`, a
//!   `#fragment` or the site root `/`. Every other string (`javascript:`, `data:`, `vbscript:`,
//!   `file:`, a protocol-relative `//host`, a bare word, the empty string) is refused.
//! - The prefixes are matched byte for byte in lowercase, so `HTTPS://host` is refused rather than
//!   normalised.
//! - A candidate holding a backslash, a control character (`U+0000`–`U+001F`,
//!   `U+007F`–`U+009F`) or an ECMA-262 whitespace character anywhere is refused. Browsers strip or
//!   delete such characters before parsing a URL and treat `\` as `/`, so refusing them keeps the
//!   checked text and the followed URL identical.
//! - The text after `https://`, `http://` and the leading `/` of a site path never starts with
//!   `/`, so no accepted candidate reaches a host other than the one it names.
//! - Each function answers exactly as its namesake in the backend's content URL policy does;
//!   `tests/safe_url.rs` holds the same case lists as the backend's tests.

/// The one absolute scheme an image may use.
const HTTPS_PREFIX: &str = "https://";
/// The plain-text absolute scheme a link may use besides [`HTTPS_PREFIX`].
const HTTP_PREFIX: &str = "http://";
/// The mail link prefix.
const MAILTO_PREFIX: &str = "mailto:";
/// A same-page fragment link.
const FRAGMENT_PREFIX: char = '#';
/// The first character of a site path.
const SITE_PATH_PREFIX: char = '/';

/// `candidate` when it is a safe image source (`https://` followed by a host, or a site path `/…`
/// other than the bare root), otherwise `None`.
///
/// The empty string is refused; a caller for which empty means "no image" tests for that first.
pub fn safe_image_src(candidate: &str) -> Option<&str> {
    is_safe_image_url(candidate).then_some(candidate)
}

/// `candidate` when it is a safe link target (`https://` or `http://` followed by a host,
/// `mailto:` followed by an address, a `#fragment` with the bare `#` included, or a site path
/// `/…` with the bare root included), otherwise `None`.
pub fn safe_link_href(candidate: &str) -> Option<&str> {
    is_safe_link_url(candidate).then_some(candidate)
}

/// Whether `candidate` is a safe link that leaves the site: an absolute `https://`, `http://` or
/// `mailto:` target. A fragment, a site path and every unsafe string answer `false`.
pub fn is_external_link(candidate: &str) -> bool {
    is_safe_link_url(candidate)
        && (candidate.starts_with(HTTPS_PREFIX)
            || candidate.starts_with(HTTP_PREFIX)
            || candidate.starts_with(MAILTO_PREFIX))
}

/// Whether `candidate` is a safe image source; see [`safe_image_src`].
fn is_safe_image_url(candidate: &str) -> bool {
    if !holds_only_url_characters(candidate) {
        return false;
    }
    if let Some(rest) = candidate.strip_prefix(HTTPS_PREFIX) {
        return opens_without_a_slash(rest);
    }
    match candidate.strip_prefix(SITE_PATH_PREFIX) {
        Some(rest) => opens_without_a_slash(rest),
        None => false,
    }
}

/// Whether `candidate` is a safe link target; see [`safe_link_href`].
fn is_safe_link_url(candidate: &str) -> bool {
    if !holds_only_url_characters(candidate) {
        return false;
    }
    if let Some(rest) = candidate
        .strip_prefix(HTTPS_PREFIX)
        .or_else(|| candidate.strip_prefix(HTTP_PREFIX))
    {
        return opens_without_a_slash(rest);
    }
    if let Some(address) = candidate.strip_prefix(MAILTO_PREFIX) {
        return !address.is_empty();
    }
    if candidate.starts_with(FRAGMENT_PREFIX) {
        return true;
    }
    match candidate.strip_prefix(SITE_PATH_PREFIX) {
        Some(rest) => rest.is_empty() || opens_without_a_slash(rest),
        None => false,
    }
}

/// Whether no character of `candidate` is one a browser strips, deletes or reinterprets while
/// parsing a URL: a backslash, a control character, or ECMA-262 whitespace.
fn holds_only_url_characters(candidate: &str) -> bool {
    !candidate
        .chars()
        .any(|c| c == '\\' || c.is_control() || is_ecma_whitespace(c))
}

/// The ECMA-262 `\s` class: Unicode `White_Space` plus the byte order mark `U+FEFF`, which
/// ECMA-262 counts as whitespace and Unicode does not.
fn is_ecma_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// Whether the text after a prefix is non-empty and does not start with `/`: a host after a
/// scheme, or a first path segment after the site root.
fn opens_without_a_slash(rest: &str) -> bool {
    rest.chars().next().is_some_and(|c| c != SITE_PATH_PREFIX)
}

#[cfg(test)]
#[path = "tests/safe_url.rs"]
mod tests;
