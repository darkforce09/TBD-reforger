//! One script prepared for the rules: its name, its lines, its outline and the banner lookups.
//!
//! **Role:** lexes and outlines a script once and answers "which documentation lines sit directly
//! above this declaration" for every rule.
//!
//! **Position:** built by [`super::check_script`] from a file name and its text; read by every
//! rule module of `enfusion_comments`.
//!
//! **Signals & state:** none; immutable after construction.
//!
//! **Invariants:**
//! - "Directly above" skips lines that hold only attributes and nothing else.
//! - A banner line is a comment-only line opened by `//!` (not `//!<`).

use super::super::enfusion_script_lexer::{CommentMarker, ScriptLine, split_script_lines};
use super::script_outline::{ScriptAttribute, ScriptItem, ScriptOutline, outline_script};

/// A lexed, outlined script.
#[derive(Debug, Clone)]
pub(crate) struct CheckedScript {
    /// The file name, such as `TBD_TaskHud.c`.
    pub(crate) file_name: String,
    /// One entry per source line; index 0 is line 1.
    pub(crate) lines: Vec<ScriptLine>,
    /// The declarations.
    pub(crate) outline: ScriptOutline,
}

impl CheckedScript {
    /// Lexes and outlines `source`, the text of the file called `file_name`.
    pub(crate) fn new(file_name: &str, source: &str) -> Self {
        let lines = split_script_lines(source);
        let outline = outline_script(&lines);
        CheckedScript {
            file_name: file_name.to_string(),
            lines,
            outline,
        }
    }

    /// The line at 1-based `number`, when it exists.
    pub(crate) fn line(&self, number: usize) -> Option<&ScriptLine> {
        number
            .checked_sub(1)
            .and_then(|index| self.lines.get(index))
    }

    /// The 1-based number of the first line above `number` that is not an attribute-only line.
    pub(crate) fn line_above(&self, number: usize) -> Option<usize> {
        (1..number)
            .rev()
            .find(|candidate| !self.outline.attribute_lines.contains(candidate))
    }

    /// The `//!` text of line `number`, when that line is a banner line.
    pub(crate) fn banner_text(&self, number: usize) -> Option<&str> {
        let line = self.line(number)?;
        if !line.is_comment_only() {
            return None;
        }
        let first = line.comments.first()?;
        (first.marker == CommentMarker::Doc && !first.after_code).then_some(first.text.as_str())
    }

    /// The banner lines directly above `line`, top to bottom, attribute-only lines skipped.
    ///
    /// Returns an empty list when the first line above is not a banner line.
    pub(crate) fn banner_above(&self, line: usize) -> Vec<(usize, &str)> {
        let mut banner = Vec::new();
        let mut cursor = self.line_above(line);
        while let Some(number) = cursor {
            match self.banner_text(number) {
                Some(text) => banner.push((number, text)),
                None => break,
            }
            cursor = self.line_above(number);
        }
        banner.reverse();
        banner
    }

    /// The banner of `item`: the banner lines directly above its first line.
    pub(crate) fn item_banner(&self, item: &ScriptItem) -> Vec<(usize, &str)> {
        self.banner_above(item.start_line)
    }

    /// The banner text line directly above `attribute`, other attribute-only lines skipped.
    pub(crate) fn tag_above_attribute(&self, attribute: &ScriptAttribute) -> Option<&str> {
        self.line_above(attribute.start_line)
            .and_then(|number| self.banner_text(number))
    }

    /// The line of the whole file as written, 1-based.
    pub(crate) fn raw_lines(&self) -> impl Iterator<Item = (usize, &str)> {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| (index + 1, line.raw.as_str()))
    }
}
