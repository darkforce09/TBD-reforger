//! Heading anchors: the fragment each heading of a page is linked by.
//!
//! **Role:** turns a heading's text into a URL fragment ([`slugify`]) and keeps the fragments of
//! one page unique ([`HeadingAnchors`]).
//! **Position:** used by the tree builder for every heading it closes, in document order; a
//! `#fragment` link in the same page reaches the heading through the anchor.
//! **Signals & state:** [`HeadingAnchors`] holds the anchors already given out for one page.
//! **Invariants:** an anchor is lowercase letters and digits joined by single hyphens, with no
//! leading or trailing hyphen, or [`EMPTY_HEADING_ANCHOR`] when the text holds no letter or
//! digit; within one page every anchor is distinct: a repeated base takes `-2`, `-3` and onward,
//! skipping any suffixed form an earlier heading already holds.

use std::collections::HashSet;

/// The anchor of a heading whose text holds no letter or digit.
pub(super) const EMPTY_HEADING_ANCHOR: &str = "section";

/// The slug of `heading_text`: letters and digits lowercased, every run of other characters
/// collapsed to one hyphen, and no hyphen at either end.
pub fn slugify(heading_text: &str) -> String {
    let mut slug = String::with_capacity(heading_text.len());
    let mut separator_pending = false;
    for character in heading_text.chars() {
        if character.is_alphanumeric() {
            if separator_pending && !slug.is_empty() {
                slug.push('-');
            }
            separator_pending = false;
            slug.extend(character.to_lowercase());
        } else {
            separator_pending = true;
        }
    }
    if slug.is_empty() {
        EMPTY_HEADING_ANCHOR.to_string()
    } else {
        slug
    }
}

/// The anchors given out so far on one page.
#[derive(Debug, Default)]
pub(super) struct HeadingAnchors {
    taken: HashSet<String>,
}

impl HeadingAnchors {
    /// The anchor of the next heading, whose text is `heading_text`: its slug, or the slug with
    /// the first free `-2`, `-3`, … suffix when an earlier heading holds it.
    pub(super) fn claim(&mut self, heading_text: &str) -> String {
        let base = slugify(heading_text);
        let mut candidate = base.clone();
        let mut suffix = 2usize;
        while self.taken.contains(&candidate) {
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
        self.taken.insert(candidate.clone());
        candidate
    }
}
