//! The content URL policy: which link targets and image sources authored content may carry.
//!
//! **Role:** decides whether a string is a safe image source ([`is_safe_image_url`]) or a safe
//! link target ([`is_safe_link_url`]) for authored content, and whether a safe link leaves the
//! site ([`is_external_link`]).
//!
//! **Position:** a `core` text rule with no caller-specific knowledge. The community-content
//! writers (the vehicle database image, the wiki markup checks at save time and the typed wiki
//! tree at render time) call it; on the wire the same rule is the `pattern` of the link `href`
//! and image `src` in `contracts/definitions/wiki-page.schema.json` and of
//! `profile_image_url` in `contracts/definitions/vehicle-database.schema.json`.
//!
//! **Signals & state:** none; pure functions over the candidate string.
//!
//! **Invariants:**
//! - The policy is an allowlist of prefixes, never a denylist of bad schemes: an image is
//!   `https://…` or a site path `/…`; a link is additionally `http://…`, `mailto:…`, a
//!   `#fragment` or the site root `/`. Every other string (`javascript:`, `data:`, `vbscript:`,
//!   `file:`, a protocol-relative `//host`, a bare word) is refused.
//! - The prefixes are matched byte for byte in lowercase, exactly as the schema patterns spell
//!   them, so `HTTPS://host` is refused rather than normalised.
//! - A candidate holding a backslash, a control character (`U+0000`–`U+001F`,
//!   `U+007F`–`U+009F`) or an ECMA-262 whitespace character anywhere is refused. Browsers strip
//!   or delete such characters before parsing a URL and treat `\` as `/`, so refusing them keeps
//!   the checked bytes and the followed URL identical.
//! - The text after `https://`, `http://` and the leading `/` of a site path never starts with
//!   `/`, so no candidate reaches a host other than the one it names.
//! - Each function accepts exactly the strings its schema pattern accepts, under ECMA-262 regular
//!   expression semantics (`tests/content_url_policy.rs` checks the two against each other).

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

/// Whether `candidate` is a safe image source: `https://` followed by a host, or a site path
/// `/…` other than the bare root.
///
/// An empty string is refused; a caller for which empty means "no image" (the vehicle database
/// `profile_image_url`) tests for that before calling.
pub fn is_safe_image_url(candidate: &str) -> bool {
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

/// Whether `candidate` is a safe link target: `https://` or `http://` followed by a host,
/// `mailto:` followed by an address, a `#fragment` (the bare `#` included), or a site path `/…`
/// (the bare root `/` included).
pub fn is_safe_link_url(candidate: &str) -> bool {
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

/// Whether `candidate` is a safe link that leaves the site: an absolute `https://`, `http://` or
/// `mailto:` target. A fragment, a site path and every unsafe string answer `false`.
pub fn is_external_link(candidate: &str) -> bool {
    is_safe_link_url(candidate)
        && (candidate.starts_with(HTTPS_PREFIX)
            || candidate.starts_with(HTTP_PREFIX)
            || candidate.starts_with(MAILTO_PREFIX))
}

/// Whether no character of `candidate` is one a browser strips, deletes or reinterprets while
/// parsing a URL: a backslash, a control character, or ECMA-262 whitespace.
fn holds_only_url_characters(candidate: &str) -> bool {
    !candidate
        .chars()
        .any(|c| c == '\\' || c.is_control() || is_ecma_whitespace(c))
}

/// The ECMA-262 `\s` class the schema patterns use: Unicode `White_Space` plus the byte order
/// mark `U+FEFF`, which ECMA-262 counts as whitespace and Unicode does not.
fn is_ecma_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// Whether the text after a prefix is non-empty and does not start with `/`: a host after a
/// scheme, or a first path segment after the site root.
fn opens_without_a_slash(rest: &str) -> bool {
    rest.chars().next().is_some_and(|c| c != SITE_PATH_PREFIX)
}

#[cfg(test)]
#[path = "tests/content_url_policy.rs"]
mod tests;
