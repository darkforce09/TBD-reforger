//! Unit coverage for the content URL policy, and its agreement with the contract patterns.
//!
//! The case tables are lists on purpose: the next scheme trick or character is one more line, and
//! a failure names the exact input. The agreement tests compile the `pattern` of the wiki link
//! `href`, the wiki image `src` and the vehicle `profile_image_url` straight out of
//! `contracts_v2/definitions/` with `regress`, the ECMA-262 engine the generated contract types
//! validate with, and require the policy to answer exactly as those patterns do.

use std::collections::BTreeSet;

use serde_json::Value;

use super::*;

const WIKI_PAGE_SCHEMA: &str =
    include_str!("../../../../../../../contracts_v2/definitions/wiki-page.schema.json");
const VEHICLE_DATABASE_SCHEMA: &str =
    include_str!("../../../../../../../contracts_v2/definitions/vehicle-database.schema.json");

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

#[test]
fn image_policy_refuses_every_unsafe_spelling() {
    for candidate in UNSAFE_EVERYWHERE.iter().chain(LINK_ONLY) {
        assert!(
            !is_safe_image_url(candidate),
            "image policy accepted {candidate:?}"
        );
    }
}

#[test]
fn image_policy_refuses_plain_http() {
    assert!(!is_safe_image_url("http://example.com/a.png"));
    assert!(is_safe_image_url("https://example.com/a.png"));
}

#[test]
fn image_policy_accepts_https_and_site_paths() {
    for candidate in SAFE_IMAGES {
        assert!(
            is_safe_image_url(candidate),
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
            !is_safe_link_url(candidate),
            "link policy accepted {candidate:?}"
        );
    }
}

#[test]
fn link_policy_accepts_web_mail_fragment_and_site_targets() {
    for candidate in SAFE_IMAGES.iter().chain(LINK_ONLY).chain(INTERNAL_LINKS) {
        assert!(
            is_safe_link_url(candidate),
            "link policy refused {candidate:?}"
        );
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

/// The one `pattern` the schema gives `property`, compiled with the ECMA-262 engine.
fn the_pattern_of(schema_text: &str, property: &str) -> regress::Regex {
    fn collect(node: &Value, property: &str, found: &mut BTreeSet<String>) {
        match node {
            Value::Object(map) => {
                if let Some(pattern) = map
                    .get("properties")
                    .and_then(|properties| properties.get(property))
                    .and_then(|schema| schema.get("pattern"))
                    .and_then(Value::as_str)
                {
                    found.insert(pattern.to_owned());
                }
                map.values()
                    .for_each(|child| collect(child, property, found));
            }
            Value::Array(items) => items
                .iter()
                .for_each(|child| collect(child, property, found)),
            _ => {}
        }
    }
    let schema: Value = serde_json::from_str(schema_text).expect("the schema is JSON");
    let mut found = BTreeSet::new();
    collect(&schema, property, &mut found);
    assert_eq!(
        found.len(),
        1,
        "`{property}` carries exactly one pattern across the schema: {found:?}"
    );
    let pattern = found.pop_first().expect("one pattern");
    regress::Regex::new(&pattern).expect("the pattern compiles")
}

/// The three contract patterns the policy must agree with.
struct ContractPatterns {
    wiki_link: regress::Regex,
    wiki_image: regress::Regex,
    vehicle_image: regress::Regex,
}

impl ContractPatterns {
    fn load() -> Self {
        Self {
            wiki_link: the_pattern_of(WIKI_PAGE_SCHEMA, "href"),
            wiki_image: the_pattern_of(WIKI_PAGE_SCHEMA, "src"),
            vehicle_image: the_pattern_of(VEHICLE_DATABASE_SCHEMA, "profile_image_url"),
        }
    }

    /// Every disagreement between the policy and the patterns on `candidate`.
    fn disagreements(&self, candidate: &str) -> Vec<String> {
        let matches = |regex: &regress::Regex| regex.find(candidate).is_some();
        let mut found = Vec::new();
        if is_safe_link_url(candidate) != matches(&self.wiki_link) {
            found.push(format!("link {candidate:?}"));
        }
        if is_safe_image_url(candidate) != matches(&self.wiki_image) {
            found.push(format!("image {candidate:?}"));
        }
        let vehicle_policy = candidate.is_empty() || is_safe_image_url(candidate);
        if vehicle_policy != matches(&self.vehicle_image) {
            found.push(format!("vehicle image {candidate:?}"));
        }
        found
    }
}

#[test]
fn agrees_with_the_contract_patterns_on_every_listed_case() {
    let patterns = ContractPatterns::load();
    let extra = [
        "mailto:",
        "MAILTO:ops@example.com",
        "http://example.com/a.png",
    ];
    let disagreements: Vec<String> = UNSAFE_EVERYWHERE
        .iter()
        .chain(SAFE_IMAGES)
        .chain(LINK_ONLY)
        .chain(INTERNAL_LINKS)
        .chain(&extra)
        .flat_map(|candidate| patterns.disagreements(candidate))
        .collect();
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}

/// Every character from `U+0000` to `U+30FF`, the non-ASCII whitespace and invisible characters
/// the ranges miss, and a private-use and an astral character, each placed where it opens a
/// host, sits inside a path, follows a prefix, or leads the value.
#[test]
fn agrees_with_the_contract_patterns_on_every_character_position() {
    let patterns = ContractPatterns::load();
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
        for candidate in [
            format!("https://{c}"),
            format!("https://a{c}b/c"),
            format!("http://{c}x"),
            format!("/{c}"),
            format!("/a{c}"),
            format!("mailto:{c}"),
            format!("#{c}"),
            format!("{c}https://a"),
            format!("{c}/a"),
            format!("{c}"),
        ] {
            disagreements.extend(patterns.disagreements(&candidate));
        }
    }
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}
