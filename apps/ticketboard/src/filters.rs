//! Composable corpus filters (T-915.2 §UI shape) — pure, unit-tested, no egui types.
//!
//! Free text searches PRECOMPUTED lowercase haystacks (id + title + summary), built
//! once per load — never per-frame formatting (design §Framework threading rule).
//! All constraints AND together; a cleared filter set matches the full measured
//! corpus (the acceptance surface: clearing restores the footer's total).

use std::collections::BTreeSet;

use tbd_tickets::{StatusName, Ticket};

use crate::board::{self, Class};
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
    /// Scope facet facts (T-918.1) — `None`/empty on programs (no `[scope]`), so
    /// any scope facet selection excludes them.
    pub domain: Option<String>,
    pub layer: Option<String>,
    pub component: Option<String>,
    pub surfaces: Vec<String>,
    /// Parsed class (absent on programs and pre-triage work tickets).
    pub class: Option<Class>,
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
                let (domain, layer, component, surfaces) = match v.scope {
                    Some(s) => (
                        Some(s.domain.as_str().to_owned()),
                        Some(s.layer.clone()),
                        s.component.clone(),
                        s.surface.clone(),
                    ),
                    None => (None, None, None, Vec::new()),
                };
                RowFacts {
                    id_lower: v.id.to_lowercase(),
                    parent_lower: v.parent.map(str::to_lowercase),
                    executor,
                    is_work: matches!(loaded.ticket, Ticket::Work(_)),
                    status: v.status.name(),
                    domain,
                    layer,
                    component,
                    surfaces,
                    class: v.class.and_then(Class::parse),
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

/// Scope facet selections (T-918.1) — one optional value per breadcrumb level.
/// `None` constrains nothing; levels AND together (and with everything else).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScopeFacets {
    pub domain: Option<String>,
    pub layer: Option<String>,
    pub component: Option<String>,
    pub surface: Option<String>,
}

impl ScopeFacets {
    pub fn any(&self) -> bool {
        self.domain.is_some()
            || self.layer.is_some()
            || self.component.is_some()
            || self.surface.is_some()
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
    /// Per-level scope facets (T-918.1) — dropdown values come from
    /// `crate::facets` (vocab ∪ corpus, narrowed top-down).
    pub scope: ScopeFacets,
    /// Class facet (T-918.1) — the closed 5-value set.
    pub class: Option<Class>,
}

impl Filters {
    pub fn is_active(&self) -> bool {
        !self.text.trim().is_empty()
            || self.executor.is_some()
            || self.kind != KindFilter::Any
            || self.statuses.iter().any(|&on| on)
            || !self.parent.trim().is_empty()
            || self.scope.any()
            || self.class.is_some()
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
            && self
                .scope
                .domain
                .as_ref()
                .is_none_or(|d| f.domain.as_ref() == Some(d))
            && self
                .scope
                .layer
                .as_ref()
                .is_none_or(|l| f.layer.as_ref() == Some(l))
            && self
                .scope
                .component
                .as_ref()
                .is_none_or(|c| f.component.as_ref() == Some(c))
            && self
                .scope
                .surface
                .as_ref()
                .is_none_or(|s| f.surfaces.contains(s))
            && self.class.is_none_or(|k| f.class == Some(k))
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

    // ---- T-918.1: scope facets + class ----

    use crate::testutil::work_scoped;

    /// Two editor tickets (different surfaces), one backend, one repo chore, one
    /// program — the facet-composition fixture.
    fn scoped_index() -> FilterIndex {
        FilterIndex::build(&corpus_of(vec![
            work_scoped(
                "T-1",
                "domain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\nsurface = [\"map_canvas\", \"toolbelt\"]",
                "class = \"bug\"\n",
            ),
            work_scoped(
                "T-2",
                "domain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\nsurface = [\"attr_panel\"]",
                "class = \"feature\"\nexecutor = \"cursor-docs\"\n",
            ),
            work_scoped(
                "T-3",
                "domain = \"website\"\nlayer = \"backend\"\ncomponent = \"http_api\"",
                "class = \"bug\"\n",
            ),
            work_scoped(
                "T-4",
                "domain = \"repo\"\nlayer = \"docs\"",
                "class = \"chore\"\n",
            ),
            program("T-9", "status = \"idea\"", &["T-9.1"]),
        ]))
    }

    #[test]
    fn index_precomputes_scope_and_class_facts() {
        let idx = scoped_index();
        assert_eq!(idx.rows[0].domain.as_deref(), Some("website"));
        assert_eq!(idx.rows[0].layer.as_deref(), Some("frontend"));
        assert_eq!(idx.rows[0].component.as_deref(), Some("mission_creator"));
        assert_eq!(idx.rows[0].surfaces, vec!["map_canvas", "toolbelt"]);
        assert_eq!(idx.rows[0].class, Some(Class::Bug));
        // Component-free scope: component None, surfaces empty.
        assert_eq!(idx.rows[3].component, None);
        assert!(idx.rows[3].surfaces.is_empty());
        // Programs: no scope facts, no class.
        assert_eq!(idx.rows[4].domain, None);
        assert_eq!(idx.rows[4].class, None);
    }

    /// Facets AND with each other, with the existing filters, and exclude
    /// programs (which have no scope to match).
    #[test]
    fn scope_facets_compose_with_existing_filters() {
        let idx = scoped_index();
        let mut filters = Filters::default();
        filters.scope.domain = Some("website".to_owned());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1", "T-2", "T-3"]);
        filters.scope.layer = Some("frontend".to_owned());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1", "T-2"]);
        filters.scope.component = Some("mission_creator".to_owned());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1", "T-2"]);
        // Surface matches MEMBERSHIP in the surface array.
        filters.scope.surface = Some("toolbelt".to_owned());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1"]);
        // Compose with an existing filter: executor now excludes T-1's default.
        filters.executor = Some("cursor-docs".to_owned());
        assert_eq!(matched_ids(&idx, &filters), Vec::<String>::new());
        // Text + facet: haystack hit AND facet hit.
        let mut filters = Filters {
            text: "title".to_owned(),
            ..Filters::default()
        };
        filters.scope.layer = Some("backend".to_owned());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-3"]);
    }

    #[test]
    fn class_filter_composes_and_clear_restores_everything() {
        let idx = scoped_index();
        let mut filters = Filters {
            class: Some(Class::Bug),
            ..Filters::default()
        };
        assert!(filters.is_active());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1", "T-3"]);
        // Class AND scope facet.
        filters.scope.layer = Some("frontend".to_owned());
        assert_eq!(matched_ids(&idx, &filters), vec!["T-1"]);
        // Class AND kind=program: nothing (programs carry no class here).
        filters.kind = KindFilter::Program;
        assert_eq!(matched_ids(&idx, &filters), Vec::<String>::new());
        // One-click clear restores the full measured count.
        filters.clear();
        assert!(!filters.is_active());
        let (verdicts, matched) = filters.apply(&idx);
        assert_eq!(matched, idx.rows.len());
        assert!(verdicts.iter().all(|&ok| ok));
    }

    /// A facet-only filter set reads as active (footer + clear button semantics).
    #[test]
    fn facet_only_filters_are_active() {
        let mut filters = Filters::default();
        assert!(!filters.is_active());
        filters.scope.surface = Some("toolbelt".to_owned());
        assert!(filters.is_active());
        filters.clear();
        filters.class = Some(Class::Docs);
        assert!(filters.is_active());
    }
}
