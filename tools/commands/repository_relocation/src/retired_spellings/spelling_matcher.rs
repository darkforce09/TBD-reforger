//! Every retired spelling of every judged manifest, and the one matcher that finds them all.
//!
//! **Role:** [`SpellingIndex`] gathers the distinct `path` row `from` spellings of all judged
//! manifests and the distinct `rust_path` rewrites with the segments of their prefixes, each with
//! the `(manifest, row)` positions that retire it. [`SpellingMatcher`] holds the path spellings
//! and the prefix segments in one Aho-Corasick automaton and scans one file with it: every start
//! of every path spelling (overlapping matches included, as
//! [`crate::path_references::path_tokens::match_starts`] finds them one spelling at a time) and
//! the set of prefix segments the file spells anywhere.
//!
//! **Position:** filled and built once per verification run by [`super::judge_manifests`] and run
//! once over each text file of [`super::tree_text::TreeText`]; the hits feed the per-row
//! judgement, which classifies each path start and runs the Rust path pass only where every
//! segment of a prefix occurs.
//!
//! **Signals & state:** none; the index is filled before the matcher is built and is read-only
//! after.
//!
//! **Invariants:** a path spelling's starts are exactly its byte-level occurrences in the file, in
//! ascending order; a prefix segment counts as present when it occurs anywhere in the file, so a
//! file missing one segment of a prefix spells that prefix nowhere (the Rust path pass matches
//! whole segments, each a slice of the file); a segment is never empty; two rows with the same
//! `from` (and, for `rust_path`, the same `to`) share one pattern and one pass result.

use std::collections::{BTreeMap, HashMap, HashSet};

use aho_corasick::{AhoCorasick, BuildError};

use crate::manifest::ManifestRow;
use crate::rust_paths::path_rules::{RustPathRules, segments_of};

/// A row's place: the manifest's index among the judged ones, and the row's index in it.
pub(super) type RowPosition = (usize, usize);

/// The distinct spellings of every judged manifest, each with the rows that retire it.
#[derive(Default)]
pub(super) struct SpellingIndex {
    /// Distinct `path` row `from` spellings.
    pub(super) path_spellings: Vec<String>,
    /// The rows that retire each path spelling.
    pub(super) path_rows: Vec<Vec<RowPosition>>,
    /// Distinct `rust_path` rewrites, one per `(from, to)`.
    pub(super) prefixes: Vec<RetiredPrefix>,
    /// The rows of each rewrite.
    pub(super) prefix_rows: Vec<Vec<RowPosition>>,
    /// Distinct prefix segments.
    segments: Vec<String>,
    path_positions: HashMap<String, usize>,
    prefix_positions: HashMap<(String, String), usize>,
    segment_positions: HashMap<String, usize>,
}

/// One distinct `rust_path` rewrite: its rules, and the segments its prefix spells.
pub(super) struct RetiredPrefix {
    /// The rules of one row of the rewrite; every row of it rewrites the same offsets.
    pub(super) rules: RustPathRules,
    /// The indexes of its prefix's segments.
    pub(super) segments: Vec<usize>,
}

impl SpellingIndex {
    /// Record the `path` row at `row`, which retires `from`.
    pub(super) fn add_path_row(&mut self, from: &str, row: RowPosition) {
        let position = match self.path_positions.get(from) {
            Some(position) => *position,
            None => {
                self.path_spellings.push(from.to_string());
                self.path_rows.push(Vec::new());
                let position = self.path_spellings.len() - 1;
                self.path_positions.insert(from.to_string(), position);
                position
            }
        };
        self.path_rows[position].push(row);
    }

    /// Record the `rust_path` row `manifest_row`, at `row`.
    pub(super) fn add_rust_path_row(&mut self, manifest_row: &ManifestRow, row: RowPosition) {
        let key = (manifest_row.from.clone(), manifest_row.to.clone());
        let position = match self.prefix_positions.get(&key) {
            Some(position) => *position,
            None => {
                let segments = segments_of(&manifest_row.from)
                    .into_iter()
                    .map(|segment| self.segment_position(segment))
                    .collect();
                self.prefixes.push(RetiredPrefix {
                    rules: RustPathRules::from_rows(std::iter::once(manifest_row)),
                    segments,
                });
                self.prefix_rows.push(Vec::new());
                let position = self.prefixes.len() - 1;
                self.prefix_positions.insert(key, position);
                position
            }
        };
        self.prefix_rows[position].push(row);
    }

    fn segment_position(&mut self, segment: String) -> usize {
        if let Some(position) = self.segment_positions.get(&segment) {
            return *position;
        }
        self.segments.push(segment.clone());
        let position = self.segments.len() - 1;
        self.segment_positions.insert(segment, position);
        position
    }
}

/// The automaton over the path spellings, then the prefix segments.
pub(super) struct SpellingMatcher {
    automaton: AhoCorasick,
    path_spellings: usize,
}

/// What one file spells: the starts of each path spelling, and the prefix segments present.
#[derive(Debug, Default)]
pub(super) struct FileHits {
    /// Each path spelling's index, with its starts in ascending order.
    pub(super) path_starts: BTreeMap<usize, Vec<usize>>,
    /// The indexes of the prefix segments the file spells.
    pub(super) segments: HashSet<usize>,
}

impl SpellingMatcher {
    /// The matcher of every path spelling and prefix segment of `index`.
    pub(super) fn new(index: &SpellingIndex) -> Result<Self, BuildError> {
        let patterns = index.path_spellings.iter().chain(&index.segments);
        let automaton = AhoCorasick::new(patterns)?;
        Ok(SpellingMatcher {
            automaton,
            path_spellings: index.path_spellings.len(),
        })
    }

    /// The hits of one file's `text`; prefix segments only when `with_segments`.
    pub(super) fn scan(&self, text: &str, with_segments: bool) -> FileHits {
        let mut hits = FileHits::default();
        for found in self.automaton.find_overlapping_iter(text) {
            let pattern = found.pattern().as_usize();
            if pattern < self.path_spellings {
                hits.path_starts
                    .entry(pattern)
                    .or_default()
                    .push(found.start());
            } else if with_segments {
                hits.segments.insert(pattern - self.path_spellings);
            }
        }
        for starts in hits.path_starts.values_mut() {
            starts.sort_unstable();
        }
        hits
    }
}
