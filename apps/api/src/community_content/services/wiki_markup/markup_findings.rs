//! The constructs a wiki save refuses, and where in the markdown each one starts.
//!
//! **Role:** names every refused construct as a [`WikiMarkupFinding`] with the 1-based source
//! line it starts on, and words its explanation for the author.
//! **Position:** filled by the tree builder while it reads a body; the save handler in
//! [`crate::community_content::handlers::wiki_knowledgebase`] answers the findings as the
//! `details.findings` of a `422 wiki_markup_refused`.
//! **Signals & state:** none; [`SourceLines`] is an immutable index built once per body.
//! **Invariants:** `line` is at least 1 and names the line the construct starts on; `detail` is
//! never empty; a quoted excerpt of the offending source holds at most [`EXCERPT_CHARACTER_LIMIT`]
//! characters, so a finding stays short whatever the body holds.
//! @contract wiki-page.schema.json#/definitions/WikiMarkupFinding

use serde::{Deserialize, Serialize};

/// The most characters of offending source a finding quotes.
pub const EXCERPT_CHARACTER_LIMIT: usize = 80;

/// One construct a save refuses.
/// @contract wiki-page.schema.json#/definitions/WikiMarkupFinding
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiMarkupFinding {
    /// The 1-based line of the body on which the construct starts.
    pub line: usize,
    /// The rule the construct breaks.
    pub code: WikiMarkupFindingCode,
    /// The explanation shown to the author.
    pub detail: String,
}

/// The rule a refused construct breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiMarkupFindingCode {
    /// A link whose target the content URL policy refuses.
    UnsafeLinkUrl,
    /// An image whose source the content URL policy refuses.
    UnsafeImageUrl,
    /// Inline or block HTML.
    RawHtml,
    /// A list, quote, callout, link or emphasis nested deeper than
    /// [`super::MAX_NESTING_DEPTH`].
    NestingTooDeep,
}

impl WikiMarkupFinding {
    /// A link to `href`, which the policy refuses, starting on `line`.
    pub fn unsafe_link(line: usize, href: &str) -> Self {
        Self {
            line,
            code: WikiMarkupFindingCode::UnsafeLinkUrl,
            detail: format!(
                "link target \"{}\" is not an https, http or mailto URL, a site path starting \
                 with / or a #fragment",
                excerpt(href)
            ),
        }
    }

    /// An image from `src`, which the policy refuses, starting on `line`.
    pub fn unsafe_image(line: usize, src: &str) -> Self {
        Self {
            line,
            code: WikiMarkupFindingCode::UnsafeImageUrl,
            detail: format!(
                "image source \"{}\" is not an https URL or a site path starting with /",
                excerpt(src)
            ),
        }
    }

    /// Raw HTML `html` starting on `line`.
    pub fn raw_html(line: usize, html: &str) -> Self {
        Self {
            line,
            code: WikiMarkupFindingCode::RawHtml,
            detail: format!(
                "raw HTML \"{}\" is not allowed; write it as markdown",
                excerpt(html.trim())
            ),
        }
    }

    /// A container opened on `line` at a depth beyond the limit.
    pub fn nesting_too_deep(line: usize) -> Self {
        Self {
            line,
            code: WikiMarkupFindingCode::NestingTooDeep,
            detail: format!(
                "lists, quotes, callouts, links and emphasis nest more than {} levels deep here",
                super::MAX_NESTING_DEPTH
            ),
        }
    }
}

/// The first [`EXCERPT_CHARACTER_LIMIT`] characters of `source`, with `…` appended when it is
/// longer.
fn excerpt(source: &str) -> String {
    let mut characters = source.chars();
    let mut quoted: String = characters.by_ref().take(EXCERPT_CHARACTER_LIMIT).collect();
    if characters.next().is_some() {
        quoted.push('…');
    }
    quoted
}

/// The byte offsets at which each line of a body starts, for turning an event's offset into a
/// 1-based line number.
pub struct SourceLines {
    /// The byte offset of the first character of every line after the first.
    line_starts: Vec<usize>,
}

impl SourceLines {
    /// Indexes the lines of `source`; a line ends at `\n`, so `\r\n` bodies count the same.
    pub fn new(source: &str) -> Self {
        let line_starts = source
            .bytes()
            .enumerate()
            .filter(|&(_, byte)| byte == b'\n')
            .map(|(index, _)| index + 1)
            .collect();
        Self { line_starts }
    }

    /// The 1-based line holding byte `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|&start| start <= offset) + 1
    }
}

#[cfg(test)]
#[path = "tests/markup_findings.rs"]
mod tests;
