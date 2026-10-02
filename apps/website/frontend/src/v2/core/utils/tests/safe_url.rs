//! Unit coverage for the app's content URL policy: the same case lists as the backend policy's
//! tests (`apps/website/api_v2/src/core/text/tests/content_url_policy.rs`), and a sweep of every
//! character in every position against the character classes of the contract patterns in
//! `contracts/definitions/wiki-page.schema.json`.
//!
//! The case tables are lists on purpose: the next scheme trick or character is one more line, and
//! a failure names the exact input.

use super::*;

/// Strings that are neither a safe image source nor a safe link target.
const UNSAFE_EVERYWHERE: &[&str] = &[
    "",
    // Script and content-bearing schemes, in every case.
    "javascript:alert(1)",
    "JaVaScRiPt:alert(1)",
    "JAVASCRIPT:alert(1)",
    "data:text/html,<script>alert(1)</script>",
    "data:image/png;base64,iVBORw0KGgo=",
    "DATA:image/svg+xml,<svg onload=alert(1)>",
    "vbscript:msgbox(1)",
    "VBScript:msgbox(1)",
    "file:///etc/passwd",
    "FILE:///C:/Windows/win.ini",
    "blob:https://example.com/1234",
    "about:blank",
    "ftp://example.com/a.png",
    "tel:+15550100",
    // Mixed-case spellings of the allowed schemes: the prefixes are matched in lowercase.
    "HTTPS://example.com/a.png",
    "Https://example.com/a.png",
    "HTTP://example.com/",
    // Protocol-relative and scheme-shaped hosts.
    "//evil.example/a.png",
    "//evil.example",
    "///evil.example/a.png",
    "https:///evil.example/a.png",
    "http:///evil.example/",
    "https:/evil.example/a.png",
    "https:evil.example/a.png",
    "https://",
    "http://",
    // Backslashes, which browsers read as `/`.
    "\\\\evil.example\\a.png",
    "/\\evil.example/a.png",
    "https:\\\\evil.example\\a.png",
    "https://example.com\\@evil.example/",
    // Whitespace around or inside the value, which browsers strip or delete.
    " https://example.com/a.png",
    "https://example.com/a.png ",
    "https://exa mple.com/a.png",
    "\thttps://example.com/a.png",
    "https://exa\tmple.com/a.png",
    "https://example.com/a.png\n",
    "\r\nhttps://example.com/a.png",
    "java\tscript:alert(1)",
    "java\nscript:alert(1)",
    " /wiki/medical-sop",
    "/wiki/medical-sop ",
    "/wiki/medical sop",
    "/\u{0}evil",
    // Control characters (C0, DEL, C1) and non-ASCII whitespace.
    "\u{0}javascript:alert(1)",
    "https://example.com/\u{0}",
    "https://example.com/\u{1b}[31m",
    "https://example.com/\u{7f}",
    "https://example.com/\u{85}",
    "https://example.com/\u{9f}",
    "https://example.com/\u{a0}",
    "https://example.com/\u{2028}",
    "https://example.com/\u{2029}",
    "https://example.com/\u{3000}",
    "https://example.com/\u{feff}",
    "\u{feff}https://example.com/a.png",
    "\u{2003}/wiki/medical-sop",
    // Relative references with no leading slash.
    "images/a.png",
    "./a.png",
    "../a.png",
    "?page=2",
    "example.com/a.png",
];

/// Safe image sources, each of which is also a safe link target.
const SAFE_IMAGES: &[&str] = &[
    "https://example.com/a.png",
    "https://cdn.example.com:8443/p/a.webp?w=640#crop",
    "https://user@example.com/a.png",
    "https://example.com",
    "https://例え.jp/画像.png",
    "https://example.com/a%20b.png",
    "/uploads/3f2b8c1e-9a4d-4e6f-8b1a-2c3d4e5f6a7b.png",
    "/a",
    "/images/../a.png",
    "/images/a.png?v=2",
];

/// Safe link targets that are not safe image sources.
const LINK_ONLY: &[&str] = &[
    "http://example.com/",
    "http://example.com/a?b=c#d",
    "mailto:ops@example.com",
    "mailto:ops@example.com?subject=Medical%20SOP",
    "mailto:/ops",
    "#",
    "#section-2",
    "#javascript:alert(1)",
    "/",
];

/// Safe links that stay on the site.
const INTERNAL_LINKS: &[&str] = &[
    "#",
    "#triage",
    "/",
    "/wiki/medical-sop",
    "/wiki/medical-sop#triage",
];

/// The backend policy's tests, whose case lists this file repeats.
const BACKEND_POLICY_TESTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../api_v2/src/core/text/tests/content_url_policy.rs"
));

/// The case lists of a policy test file: from the first list's doc line to the end of
/// `INTERNAL_LINKS`.
fn case_lists(test_file: &str) -> &str {
    let start = test_file
        .find("/// Strings that are neither")
        .expect("the case lists open with UNSAFE_EVERYWHERE");
    let internal = start
        + test_file[start..]
            .find("const INTERNAL_LINKS")
            .expect("the case lists close with INTERNAL_LINKS");
    let end = internal
        + test_file[internal..]
            .find("];")
            .expect("INTERNAL_LINKS is closed");
    &test_file[start..end]
}

/// A case the backend adds to its lists must be added here too, or the two policies may drift
/// apart on exactly that case.
#[test]
fn the_case_lists_match_the_backend_policy_tests() {
    assert_eq!(
        case_lists(include_str!("safe_url.rs")),
        case_lists(BACKEND_POLICY_TESTS),
        "copy the case lists of apps/website/api_v2/src/core/text/tests/content_url_policy.rs"
    );
}

/// The image policy under its boolean name, for the case tables.
fn is_safe_image(candidate: &str) -> bool {
    safe_image_src(candidate).is_some()
}

/// The link policy under its boolean name, for the case tables.
fn is_safe_link(candidate: &str) -> bool {
    safe_link_href(candidate).is_some()
}

#[test]
fn a_safe_candidate_is_answered_unchanged() {
    assert_eq!(
        safe_image_src("https://example.com/a.png"),
        Some("https://example.com/a.png")
    );
    assert_eq!(safe_link_href("#triage"), Some("#triage"));
    assert_eq!(safe_link_href("javascript:alert(1)"), None);
    assert_eq!(safe_image_src(""), None);
}

#[test]
fn image_policy_refuses_every_unsafe_spelling() {
    for candidate in UNSAFE_EVERYWHERE.iter().chain(LINK_ONLY) {
        assert!(
            !is_safe_image(candidate),
            "image policy accepted {candidate:?}"
        );
    }
}

#[test]
fn image_policy_refuses_plain_http() {
    assert!(!is_safe_image("http://example.com/a.png"));
    assert!(is_safe_image("https://example.com/a.png"));
}

#[test]
fn image_policy_accepts_https_and_site_paths() {
    for candidate in SAFE_IMAGES {
        assert!(
            is_safe_image(candidate),
            "image policy refused {candidate:?}"
        );
    }
}

#[test]
fn link_policy_refuses_every_unsafe_spelling() {
    for candidate in UNSAFE_EVERYWHERE
        .iter()
        .chain(&["mailto:", "MAILTO:ops@example.com"])
    {
        assert!(
            !is_safe_link(candidate),
            "link policy accepted {candidate:?}"
        );
    }
}

#[test]
fn link_policy_accepts_web_mail_fragment_and_site_targets() {
    for candidate in SAFE_IMAGES.iter().chain(LINK_ONLY).chain(INTERNAL_LINKS) {
        assert!(is_safe_link(candidate), "link policy refused {candidate:?}");
    }
}

#[test]
fn external_links_are_the_absolute_web_and_mail_targets() {
    for candidate in [
        "https://example.com/",
        "http://example.com/",
        "mailto:ops@example.com",
    ] {
        assert!(is_external_link(candidate), "{candidate:?} is external");
    }
    for candidate in INTERNAL_LINKS {
        assert!(!is_external_link(candidate), "{candidate:?} is internal");
    }
    for candidate in UNSAFE_EVERYWHERE {
        assert!(!is_external_link(candidate), "{candidate:?} is not a link");
    }
}

/// The characters the contract patterns exclude everywhere, transcribed from their classes rather
/// than computed the way the policy computes them: `\\`, `\x00-\x1F`, `\x7F-\x9F`, and the
/// ECMA-262 `\s` class (its WhiteSpace and LineTerminator code points, the `Zs` category among
/// them).
fn the_patterns_exclude(c: char) -> bool {
    const ECMA_WHITESPACE: &[char] = &[
        '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{00A0}',
        '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
        '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}', '\u{2029}',
        '\u{202F}', '\u{205F}', '\u{3000}', '\u{FEFF}',
    ];
    c == '\\'
        || ('\u{00}'..='\u{1F}').contains(&c)
        || ('\u{7F}'..='\u{9F}').contains(&c)
        || ECMA_WHITESPACE.contains(&c)
}

/// Every character from `U+0000` to `U+30FF`, the invisible characters the ranges miss, and a
/// private-use and an astral character, each placed where it opens a host, sits inside a path,
/// follows `mailto:` or `#`, or opens a site path.
#[test]
fn every_character_in_every_position_answers_as_the_contract_patterns_do() {
    let extra = [
        '\u{180E}',
        '\u{200B}',
        '\u{205F}',
        '\u{2060}',
        '\u{3000}',
        '\u{E000}',
        '\u{FEFF}',
        '\u{FFFD}',
        '\u{1F600}',
        '\u{10FFFF}',
    ];
    let mut disagreements = Vec::new();
    for c in (0u32..0x3100).filter_map(char::from_u32).chain(extra) {
        let allowed = !the_patterns_exclude(c);
        let opens = allowed && c != '/';
        // (candidate, image expected, link expected)
        for (candidate, image, link) in [
            (format!("https://{c}"), opens, opens),
            (format!("https://a{c}b/c"), allowed, allowed),
            (format!("http://{c}x"), false, opens),
            (format!("/{c}"), opens, opens),
            (format!("/a{c}"), allowed, allowed),
            (format!("mailto:{c}"), false, allowed),
            (format!("#{c}"), false, allowed),
        ] {
            if is_safe_image(&candidate) != image {
                disagreements.push(format!("image {candidate:?}"));
            }
            if is_safe_link(&candidate) != link {
                disagreements.push(format!("link {candidate:?}"));
            }
        }
    }
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}
