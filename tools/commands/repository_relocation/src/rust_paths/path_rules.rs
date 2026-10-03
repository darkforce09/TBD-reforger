//! The `rust_path` rows as segment rules, and how one path is rewritten by them.
//!
//! **Role:** holds the rows as [`RustPathRule`]s, longest prefix first, and rewrites one path's
//! segments: an absolute path by its longest matching prefix, a `self::` or `super::` path by
//! first turning it into an absolute `crate::` path when it crosses the edge of a moved module
//! (from inside the module to outside it, or the reverse), since the move changes the module depth
//! the relative path was counted against.
//!
//! **Position:** used by the Rust path pass ([`super`]) for `use` leaves, code paths, comments and
//! string literals, and by the verification ([`super::super::retired_spellings`]).
//!
//! **Signals & state:** none; immutable rules.
//!
//! **Invariants:** a rule matches whole segments only; the longest matching `from` wins; a relative
//! path whose module and target sit on the same side of every moved module is left as written,
//! since it moves with them; a relative path that climbs above the crate is left alone.

use super::super::manifest::{ManifestRow, RowKind};

/// One `rust_path` row as segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RustPathRule {
    /// The row's manifest line.
    pub(crate) row_line: usize,
    /// The retired prefix's segments, without the trailing `::`.
    pub(crate) from: Vec<String>,
    /// The new prefix's segments.
    pub(crate) to: Vec<String>,
}

impl RustPathRule {
    /// The rule of one `rust_path` row.
    pub(crate) fn from_row(row: &ManifestRow) -> RustPathRule {
        RustPathRule {
            row_line: row.line,
            from: segments_of(&row.from),
            to: segments_of(&row.to),
        }
    }

    /// The retired prefix without its trailing `::`, as text.
    pub(crate) fn retired_prefix_text(&self) -> String {
        self.from.join("::")
    }

    /// The new prefix without its trailing `::`, as text.
    pub(crate) fn new_prefix_text(&self) -> String {
        self.to.join("::")
    }

    /// The module the rule moves, below `crate`, when its prefix starts with `crate`.
    fn moved_module(&self) -> Option<&[String]> {
        (self.from.first().map(String::as_str) == Some("crate")).then(|| &self.from[1..])
    }
}

/// A set of rules, longest `from` first.
#[derive(Clone, Debug, Default)]
pub(crate) struct RustPathRules {
    rules: Vec<RustPathRule>,
}

impl RustPathRules {
    /// The rules of every `rust_path` row among `rows`.
    pub(crate) fn from_rows<'a, I: IntoIterator<Item = &'a ManifestRow>>(rows: I) -> RustPathRules {
        let mut rules: Vec<RustPathRule> = rows
            .into_iter()
            .filter(|row| row.kind == RowKind::RustPath)
            .map(RustPathRule::from_row)
            .collect();
        rules.sort_by(|a, b| b.from.len().cmp(&a.from.len()).then(a.from.cmp(&b.from)));
        RustPathRules { rules }
    }

    /// The rules, longest first.
    pub(crate) fn rules(&self) -> &[RustPathRule] {
        &self.rules
    }

    /// Whether there is no rule.
    pub(crate) fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Whether any rule moves a `crate::` module, which makes relative paths worth resolving.
    pub(crate) fn moves_crate_modules(&self) -> bool {
        self.rules.iter().any(|rule| rule.moved_module().is_some())
    }

    /// The rule whose `from` is the longest prefix of `segments`.
    pub(crate) fn matching(&self, segments: &[String]) -> Option<&RustPathRule> {
        self.rules.iter().find(|rule| {
            segments.len() >= rule.from.len() && segments[..rule.from.len()] == rule.from[..]
        })
    }

    /// `segments` rewritten, with the row that rewrote them, or `None` when no rule applies.
    /// `module` is the module path (below `crate`) the path is written in, needed for `self::`
    /// and `super::` paths.
    pub(crate) fn rewrite(
        &self,
        segments: &[String],
        module: Option<&[String]>,
    ) -> Option<(usize, Vec<String>)> {
        let first = segments.first()?.as_str();
        if first == "self" || first == "super" {
            let absolute = self.absolute_if_crossing(segments, module?)?;
            return Some(match self.rewrite_absolute(&absolute.1) {
                Some((row_line, rewritten)) => (row_line, rewritten),
                None => absolute,
            });
        }
        self.rewrite_absolute(segments)
    }

    fn rewrite_absolute(&self, segments: &[String]) -> Option<(usize, Vec<String>)> {
        let rule = self.matching(segments)?;
        let mut rewritten = rule.to.clone();
        rewritten.extend(segments[rule.from.len()..].iter().cloned());
        Some((rule.row_line, rewritten))
    }

    /// The `crate::` form of a relative path that crosses the edge of a moved module.
    fn absolute_if_crossing(
        &self,
        segments: &[String],
        module: &[String],
    ) -> Option<(usize, Vec<String>)> {
        let lead = usize::from(segments[0] == "self");
        let climbs = segments[lead..]
            .iter()
            .take_while(|segment| *segment == "super")
            .count();
        let consumed = lead + climbs;
        if climbs > module.len() || consumed == segments.len() {
            return None;
        }
        let mut target: Vec<String> = module[..module.len() - climbs].to_vec();
        target.extend(segments[consumed..].iter().cloned());
        let crossed = self.rules.iter().find(|rule| {
            rule.moved_module()
                .is_some_and(|moved| module.starts_with(moved) != target.starts_with(moved))
        })?;
        let mut absolute = vec!["crate".to_string()];
        absolute.extend(target);
        Some((crossed.row_line, absolute))
    }
}

/// The segments of a `::`-separated prefix, a trailing `::` ignored.
pub(crate) fn segments_of(prefix: &str) -> Vec<String> {
    prefix
        .trim_end_matches("::")
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
