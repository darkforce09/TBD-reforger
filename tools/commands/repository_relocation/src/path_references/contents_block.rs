//! The tree lines of a README's Contents block: the spans a frozen area's index keeps live.
//!
//! **Role:** finds a README's Contents block, the first fenced code block whose info string is
//! exactly `text` that opens after the `## Contents` heading and before the next `## ` heading, as
//! the README standard defines it, and returns the tree part of each of its lines: the root line's
//! folder path and each entry line's drawing and name, up to the two spaces that open its role.
//!
//! **Position:** read by [`super::allowed_spans`], which opens these spans beside the link
//! destinations of a frozen area's `README.md`
//! ([`crate::file_treatment::FileTreatment::FrozenIndex`]), so a move rewrites the paths the tree
//! names while the roles, prose written when the record was made, stay as written.
//!
//! **Signals & state:** none; pure functions over the document text.
//!
//! **Invariants:** a heading or a fence inside another fenced block is never taken for the
//! Contents heading or block; only the lines between the fences are read (to the end of the text
//! when no fence closes the block), so the fences themselves are never opened; a span never
//! reaches a line's role or its line break.

use std::ops::Range;

use super::markdown_links::fence_marker;

/// The heading the Contents block follows.
const CONTENTS_HEADING: &str = "## Contents";

/// How a heading that ends the Contents section begins.
const SECTION_HEADING: &str = "## ";

/// The info string on the Contents block's opening fence.
const CONTENTS_INFO_STRING: &str = "text";

/// What separates an entry from its role: two spaces, and any more that follow.
const ROLE_SEPARATOR: &str = "  ";

/// The characters of the tree drawing before an entry's name.
const TREE_DRAWING: [char; 5] = [' ', '│', '├', '└', '─'];

/// The tree part of every line of the Contents block of `source`: from the line's start through
/// the root folder or the entry name; empty when the README has no Contents block.
pub(crate) fn contents_tree_lines(source: &str) -> Vec<Range<usize>> {
    let Some(block) = contents_block(source) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    let mut offset = block.start;
    for line in source[block].split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let content = line.trim_end_matches(['\n', '\r']);
        let name_start = content.len() - content.trim_start_matches(TREE_DRAWING).len();
        let name_end = content[name_start..]
            .find(ROLE_SEPARATOR)
            .map_or(content.len(), |end| name_start + end);
        if name_end > name_start {
            spans.push(line_start..line_start + name_end);
        }
    }
    spans
}

/// An open fenced block: its fence character, its fence length, and where its body starts when it
/// is the Contents block.
type OpenFence = ((u8, usize), Option<usize>);

/// The span of the lines inside the Contents block of `source`, when it has one.
fn contents_block(source: &str) -> Option<Range<usize>> {
    let mut offset = 0;
    let mut heading_seen = false;
    let mut open: Option<OpenFence> = None;
    for line in source.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let trimmed = line.trim_start();
        let marker = fence_marker(trimmed);
        if let Some(((byte, length), body_start)) = open {
            let closes = marker.is_some_and(|(found, run)| {
                found == byte && run >= length && trimmed[run..].trim().is_empty()
            });
            if closes {
                if let Some(body_start) = body_start {
                    return Some(body_start..line_start);
                }
                open = None;
            }
            continue;
        }
        if let Some((byte, run)) = marker {
            let is_contents = heading_seen && trimmed[run..].trim() == CONTENTS_INFO_STRING;
            open = Some(((byte, run), is_contents.then_some(offset)));
            continue;
        }
        if line.starts_with(SECTION_HEADING) {
            if heading_seen {
                return None;
            }
            heading_seen = line.trim_end() == CONTENTS_HEADING;
        }
    }
    open.and_then(|(_, body_start)| body_start)
        .map(|body_start| body_start..source.len())
}
