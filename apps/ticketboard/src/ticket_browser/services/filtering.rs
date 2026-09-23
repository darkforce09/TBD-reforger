//! Composable ticket filters over facts precomputed when the corpus loads.
//!
//! Applying filters changes projections, never registry data. Clearing them restores
//! the full ticket count; empty status selections mean all statuses.

use std::collections::BTreeSet;

use ticket_engine::{StatusName, Ticket};

use crate::ticket_registry::models::corpus::Corpus;
use crate::ticket_registry::models::projection::{self as board, Class};

/// Per-ticket filter facts, precomputed once per load.
pub struct RowFacts {
    pub id_lower: String,
    /// Explicit `parent` field (work tickets), lowercased.
    pub parent_lower: Option<String>,
    /// Default-applied executor label (absent == claude-code).
    pub executor: String,
    pub is_work: bool,
    pub status: StatusName,
    /// Scope facet facts — `None`/empty on programs (no `[scope]`), so
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

/// Scope facet selections — one optional value per breadcrumb level.
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
    /// Per-level scope facets — dropdown values come from
    /// `crate::ticket_browser::services::scope_facets` (vocab ∪ corpus, narrowed top-down).
    pub scope: ScopeFacets,
    /// Class facet — the closed 5-value set.
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
#[path = "tests/filtering.rs"]
mod tests;
