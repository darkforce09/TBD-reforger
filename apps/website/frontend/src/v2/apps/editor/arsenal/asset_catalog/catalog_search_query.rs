//! Asset catalog catalog search query behavior.

use super::*;

/// Catalog data field selected by a search operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchField {
    Label,
    ClassName,
    Mod,
}

/// Pattern matcher selected by a catalog query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchPattern {
    All,
    Pending,
    Plain(String),
    Glob(GlobPattern),
    Regex(Rx),
    Invalid,
}

/// Parsed catalog field and matching pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchQuery {
    pub field: SearchField,
    pub pattern: SearchPattern,
}

const OPERATORS: &[(&str, SearchField)] = &[
    ("class:", SearchField::ClassName),
    ("mod:", SearchField::Mod),
    ("mod ", SearchField::Mod),
];

/// Parses leading search operators and pattern syntax.
#[must_use]
pub fn parse_search_query(query: &str) -> SearchQuery {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return SearchQuery {
            field: SearchField::Label,
            pattern: SearchPattern::All,
        };
    }
    let (field, body) = OPERATORS
        .iter()
        .find_map(|(tok, field)| strip_prefix_ci(trimmed, tok).map(|rest| (*field, rest)))
        .unwrap_or((SearchField::Label, trimmed));
    SearchQuery {
        field,
        pattern: parse_search_pattern(body.trim()),
    }
}

/// Selects the supported matcher for a search pattern body.
pub(super) fn parse_search_pattern(body: &str) -> SearchPattern {
    if body.is_empty() {
        return SearchPattern::Pending;
    }
    if let Some(inner) = regex_body(body) {
        if inner.is_empty() {
            return SearchPattern::Pending;
        }
        return Rx::parse(inner).map_or(SearchPattern::Invalid, SearchPattern::Regex);
    }
    if body.contains('*') || body.contains('?') {
        return SearchPattern::Glob(GlobPattern::parse(body));
    }
    SearchPattern::Plain(body.to_lowercase())
}

fn regex_body(body: &str) -> Option<&str> {
    let b = body.as_bytes();
    (b.len() >= 2 && b[0] == b'/' && b[b.len() - 1] == b'/').then(|| &body[1..body.len() - 1])
}

impl SearchPattern {
    /// Checks whether a pattern matches a chosen field value.
    pub(super) fn hits(&self, hay: &str, prefix: bool) -> bool {
        match self {
            SearchPattern::Plain(q) => {
                let lower = hay.to_lowercase();
                if prefix {
                    lower.starts_with(q.as_str())
                } else {
                    lower.contains(q.as_str())
                }
            }
            SearchPattern::Glob(g) => g.matches(hay),
            SearchPattern::Regex(r) => r.is_match(hay),
            SearchPattern::All | SearchPattern::Pending | SearchPattern::Invalid => false,
        }
    }
}

/// Explains an empty result or unfinished search pattern.
#[must_use]
pub fn search_empty_message(query: &str, noun: &str) -> String {
    let q = parse_search_query(query);
    match (q.field, &q.pattern) {
        (SearchField::ClassName, SearchPattern::Pending) => {
            "Type a class name after class:".to_string()
        }
        (SearchField::Mod, SearchPattern::Pending) => "Type a mod name after mod:".to_string(),
        (_, SearchPattern::Pending) => "Type a pattern between the slashes.".to_string(),
        (_, SearchPattern::Invalid) => {
            "That /…/ pattern could not be read — check the brackets and parentheses.".to_string()
        }
        _ => format!("No {noun} match."),
    }
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix)
    {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}
