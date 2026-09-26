use super::*;
use crate::test_support::{corpus_of, program, work};

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
            "executor = \"documentation\"\n",
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
    assert_eq!(idx.executors, vec!["claude-code", "documentation"]);
    assert_eq!(idx.rows[1].executor, "documentation");
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
    // executor + status: and (the documentation row and the shipped row drop out).
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
    // itself, dotted descendants, explicit-parent children — never (dot-boundary) and never.
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

// ---- scope facets + class ----

use crate::test_support::work_scoped;

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
            "class = \"feature\"\nexecutor = \"documentation\"\n",
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
    // Compose with an existing filter: executor now excludes 's default.
    filters.executor = Some("documentation".to_owned());
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
