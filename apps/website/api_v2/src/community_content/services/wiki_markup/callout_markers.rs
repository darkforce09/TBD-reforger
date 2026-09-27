//! Callout markers: which block quotes are callouts, and of which kind.
//!
//! **Role:** maps a GitHub alert kind that pulldown-cmark recognised to a [`WikiCalloutKind`]
//! ([`alert_callout_kind`]), and recognises the bracket marker `[!KIND]` that opens a plain block
//! quote ([`take_bracket_marker`]).
//! **Position:** called by the tree builder when it closes a block quote.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a bracket marker counts only as the first text of the quote's first
//! paragraph, spelled `[!CRITICAL]`, `[!CAUTION]`, `[!WARNING]`, `[!TIP]`, `[!NOTE]` or
//! `[!INFO]` in any letter case, and followed by whitespace or nothing; a recognised marker is
//! removed with the whitespace after it, and a paragraph left empty is removed; any other quote
//! is left unchanged.

use pulldown_cmark::BlockQuoteKind;

use super::document_tree::{WikiBlock, WikiCalloutKind, WikiInline};

/// The bracket marker tags and the callout kind each opens, matched without regard to case.
const BRACKET_MARKER_KINDS: [(&str, WikiCalloutKind); 6] = [
    ("CRITICAL", WikiCalloutKind::Critical),
    ("CAUTION", WikiCalloutKind::Caution),
    ("WARNING", WikiCalloutKind::Warning),
    ("TIP", WikiCalloutKind::Tip),
    ("NOTE", WikiCalloutKind::Note),
    ("INFO", WikiCalloutKind::Info),
];

/// The callout kind of a GitHub alert (`> [!NOTE]` alone on the quote's first line).
pub fn alert_callout_kind(kind: BlockQuoteKind) -> WikiCalloutKind {
    match kind {
        BlockQuoteKind::Note => WikiCalloutKind::Note,
        BlockQuoteKind::Tip => WikiCalloutKind::Tip,
        BlockQuoteKind::Important => WikiCalloutKind::Important,
        BlockQuoteKind::Warning => WikiCalloutKind::Warning,
        BlockQuoteKind::Caution => WikiCalloutKind::Caution,
    }
}

/// The callout kind named by a bracket marker opening the quote whose blocks are `blocks`, with
/// the marker removed from them; `None`, with `blocks` untouched, when the quote opens with no
/// marker.
pub fn take_bracket_marker(blocks: &mut Vec<WikiBlock>) -> Option<WikiCalloutKind> {
    let Some(WikiBlock::Paragraph { inlines }) = blocks.first_mut() else {
        return None;
    };
    let Some(WikiInline::Text { text }) = inlines.first_mut() else {
        return None;
    };
    let (kind, marker_length) = bracket_marker(text)?;
    let remainder = text[marker_length..].trim_start().to_string();
    if remainder.is_empty() {
        inlines.remove(0);
        if matches!(inlines.first(), Some(WikiInline::LineBreak)) {
            inlines.remove(0);
        }
    } else {
        *text = remainder;
    }
    if inlines.is_empty() {
        blocks.remove(0);
    }
    Some(kind)
}

/// The kind and the byte length of the bracket marker `text` opens with, if it opens with one
/// followed by whitespace or nothing.
fn bracket_marker(text: &str) -> Option<(WikiCalloutKind, usize)> {
    let inner = text.strip_prefix("[!")?;
    let close = inner.find(']')?;
    let tag = &inner[..close];
    let kind = BRACKET_MARKER_KINDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(tag))
        .map(|&(_, kind)| kind)?;
    let marker_length = "[!".len() + close + "]".len();
    let followed_by_space = text[marker_length..]
        .chars()
        .next()
        .is_none_or(char::is_whitespace);
    followed_by_space.then_some((kind, marker_length))
}

#[cfg(test)]
#[path = "tests/callout_markers.rs"]
mod tests;
