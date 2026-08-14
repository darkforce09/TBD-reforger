//! Composable corpus filters (T-915.2 §UI shape) — pure, unit-tested, no egui types.
//!
//! Free text searches PRECOMPUTED lowercase haystacks (id + title + summary), built
//! once per load — never per-frame formatting (design §Framework threading rule).
//! All constraints AND together; a cleared filter set matches the full measured
//! corpus (the acceptance surface: clearing restores the footer's total).

use std::collections::BTreeSet;

use tbd_tickets::{StatusName, Ticket};

use crate::board;
use crate::corpus::Corpus;

/// Per-ticket filter facts, precomputed once per load.
pub struct RowFacts {
    pub id_lower: String,
    /// Explicit `parent` field (work tickets), lowercased.
    pub parent_lower: Option<String>,
    /// Default-applied executor label (absent == claude-code).
    pub executor: String,
    pub is_work: bool,
    pub status: StatusName,
    /// Lowercase `id \n title \n summary`.
    pub haystack: String,
}

pub struct FilterIndex {
    pub rows: Vec<RowFacts>,
    /// Distinct executor labels, sorted — the executor dropdown options.
    pub executors: Vec<String>,
}

impl FilterIndex {
    pub fn build(corpus: &Corpus) -> Self {
        let mut executors = BTreeSet::new();
        let rows = corpus
            .tickets
            .iter()
            .map(|loaded| {
                let v = board::view(&loaded.ticket);
                let executor = board::executor_label(v.executor);
                executors.insert(executor.clone());
                RowFacts {
                    id_lower: v.id.to_lowercase(),
                    parent_lower: v.parent.map(str::to_lowercase),
                    executor,
                    is_work: matches!(loaded.ticket, Ticket::Work(_)),
                    status: v.status.name(),
                    haystack: format!("{}\n{}\n{}", v.id, v.title, v.summary).to_lowercase(),
                }
            })
            .collect();
        Self {
            rows,
            executors: executors.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KindFilter {
    #[default]
    Any,
    Work,
    Program,
}

impl KindFilter {
    pub const ALL: [KindFilter; 3] = [KindFilter::Any, KindFilter::Work, KindFilter::Program];

    pub fn label(self) -> &'static str {
        match self {
            KindFilter::Any => "any kind",
            KindFilter::Work => "work",
            KindFilter::Program => "program",
        }
    }
}

/// The composable filter set. Default fields constrain nothing; `statuses` with NO
/// toggle on means "all statuses" — so one-click Clear restores the full count.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filters {
    pub text: String,
    pub executor: Option<String>,
    pub kind: KindFilter,
    /// Multi-toggle in `board::STATUS_ORDER` (== `column_of`) index space.
    pub statuses: [bool; 8],
    /// Program/parent id: the id itself, its dotted descendants, or an explicit
    /// `parent` naming it.
    pub parent: String,
}

impl Filters {
    pub fn is_active(&self) -> bool {
        !self.text.trim().is_empty()
            || self.executor.is_some()
            || self.kind != KindFilter::Any
            || self.statuses.iter().any(|&on| on)
            || !self.parent.trim().is_empty()
    }

    /// One-click clear — every constraint off.
    pub fn clear(&mut self) {
        *self = Filters::default();
    }

    /// Apply to the whole index: per-row verdicts plus the match count. Needles are
    /// lowercased once here, not per row.
    pub fn apply(&self, index: &FilterIndex) -> (Vec<bool>, usize) {
        let text = self.text.trim().to_lowercase();
        let parent = self.parent.trim().to_lowercase();
        let any_status = self.statuses.iter().any(|&on| on);
        let verdicts: Vec<bool> = index
            .rows
            .iter()
            .map(|facts| self.row_matches(facts, &text, &parent, any_status))
            .collect();
        let count = verdicts.iter().filter(|&&ok| ok).count();
        (verdicts, count)
    }

    fn row_matches(&self, f: &RowFacts, text: &str, parent: &str, any_status: bool) -> bool {
        (text.is_empty() || f.haystack.contains(text))
            && self.executor.as_ref().is_none_or(|e| f.executor == *e)
            && match self.kind {
                KindFilter::Any => true,
                KindFilter::Work => f.is_work,
                KindFilter::Program => !f.is_work,
            }
            && (!any_status || self.statuses[board::column_of(f.status)])
            && (parent.is_empty() || parent_matches(f, parent))
    }
}

/// The id itself, a dotted descendant (`t-915` matches `t-915.2`, never `t-9150`),
/// or an explicit `parent` naming it — case-insensitive.
fn parent_matches(f: &RowFacts, parent: &str) -> bool {
    f.id_lower == parent
        || f.id_lower
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('.'))
        || f.parent_lower.as_deref() == Some(parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{corpus_of, program, work};

    fn index() -> FilterIndex {
        FilterIndex::build(&corpus_of(vec![
            work(
                "T-1",
                "status = \"queued\"\norder = 10",
                "summary = \"red dawn seeding\"\n",
            ),
            work(
                "T-2",
                "status = \"queued\"\norder = 20",
                "executor = \"cursor-docs\"\n",
            ),
            work("T-3", "status = \"shipped\"", ""),
            program("T-9", "status = \"queued\"\norder = 30", &["T-9.1"]),
            work("T-9.1", "status = \"idea\"", "parent = \"T-9\"\n"),
            program("T-9.2", "status = \"idea\"", &["T-9.2.1"]),
            work("T-90", "status = \"idea\"", ""),
        ]))
    }

    fn matched_ids(index: &FilterIndex, filters: &Filters) -> Vec<String> {
        let (verdicts, count) = filters.apply(index);
        let ids: Vec<String> = index
            .rows
            .iter()
            .zip(&verdicts)
            .filter(|(_, ok)| **ok)
            .map(|(f, _)| f.id_lower.to_uppercase())
            .collect();
        assert_eq!(ids.len(), count);
        ids
    }

    #[test]
    fn index_precomputes_lowercase_haystacks_and_executors() {
        let idx = index();
        assert!(idx.rows[0].haystack.contains("t-1"));
        assert!(idx.rows[0].haystack.contains("red dawn"));
        assert_eq!(idx.executors, vec!["claude-code", "cursor-docs"]);
        assert_eq!(idx.rows[1].executor, "cursor-docs");
        assert_eq!(idx.rows[0].executor, "claude-code");
        assert!(!idx.rows[3].is_work);
        assert_eq!(idx.rows[4].parent_lower.as_deref(), Some("t-9"));
    }

    #[test]
    fn filters_compose_as_intersection() {
        let idx = index();
        let mut filters = Filters {
            executor: Some("claude-code".to_string()),
            ..Filters::default()
        };
        filters.statuses[board::column_of(StatusName::Queued)] = true;
        // executor + status: T-1 and T-9 (T-2 is cursor-docs, T-3 shipped).
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1", "T-9"]);
        // + kind work: drops the program.
        filters.kind = KindFilter::Work;
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1"]);
        // + free text that misses: nothing.
        filters.text = "no such words".to_string();
        assert_eq!(matched_ids(&idx, &filters), Vec::<String>::new());
        // Free text is case-insensitive over the precomputed haystack.
        filters.text = "RED Dawn".to_string();
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1"]);
    }

    #[test]
    fn clear_restores_the_full_measured_count() {
        let idx = index();
        let mut filters = Filters {
            text: "dawn".to_string(),
            kind: KindFilter::Work,
            ..Filters::default()
        };
        filters.statuses[0] = true;
        assert!(filters.is_active());
        let (_, matched) = filters.apply(&idx);
        assert!(matched < idx.rows.len());
        filters.clear();
        assert!(!filters.is_active());
        let (verdicts, matched) = filters.apply(&idx);
        assert_eq!(matched, idx.rows.len());
        assert!(verdicts.iter().all(|&ok| ok));
    }

    #[test]
    fn no_status_toggle_means_all_statuses() {
        let idx = index();
        let filters = Filters::default();
        assert!(!filters.is_active());
        let (_, matched) = filters.apply(&idx);
        assert_eq!(matched, idx.rows.len());
    }

    #[test]
    fn parent_filter_matches_the_subtree_only() {
        let idx = index();
        let filters = Filters {
            parent: "t-9".to_string(),
            ..Filters::default()
        };
        // T-9 itself, dotted descendants, explicit-parent children — never T-90
        // (dot-boundary) and never T-1.
        assert_eq!(matched_ids(&idx, &filters), vec!["T-9", "T-9.1", "T-9.2"]);
        // Case-insensitive with surrounding whitespace trimmed.
        let filters = Filters {
            parent: " T-9 ".to_string(),
            ..Filters::default()
        };
        assert_eq!(matched_ids(&idx, &filters), vec!["T-9", "T-9.1", "T-9.2"]);
    }

    #[test]
    fn kind_filter_splits_work_and_program() {
        let idx = index();
        let filters = Filters {
            kind: KindFilter::Program,
            ..Filters::default()
        };
        assert_eq!(matched_ids(&idx, &filters), vec!["T-9", "T-9.2"]);
    }
}
