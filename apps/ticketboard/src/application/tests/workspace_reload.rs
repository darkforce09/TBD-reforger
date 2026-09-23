use super::*;
use crate::test_support::{corpus_of, work};

fn bundle(ids: &[&str]) -> LoadBundle {
    LoadBundle {
        corpus: Ok(corpus_of(
            ids.iter()
                .map(|id| work(id, "status = \"idea\"", ""))
                .collect(),
        )),
        lock: LockState::Missing {
            message: "missing plan".into(),
        },
        metrics: MetricsState::NoReceipts,
        estimates: estimates::RawEstimates::default(),
        vocab: None,
    }
}

#[test]
fn reload_resolves_selection_by_id_and_preserves_filters_and_independent_sorts() {
    let mut previous = WorkspaceState::reload(bundle(&["T-1", "T-2"]), None).unwrap();
    previous.selected = Some(0);
    previous.compare = Some(1);
    previous.legacy_expanded = true;
    previous.filters.text = "T-1".into();
    previous.metrics_sort.ticket = previous.metrics_sort.ticket.toggled(metrics::SortKey::Runs);
    previous.metrics_sort.agent = previous
        .metrics_sort
        .agent
        .toggled(metrics::SortKey::Elapsed);
    previous.est_sort.class = previous
        .est_sort
        .class
        .toggled(estimates::EstimatedSortKey::Tickets);
    previous.est_sort.domain = previous
        .est_sort
        .domain
        .toggled(estimates::EstimatedSortKey::DiffLoc);
    previous.refilter();

    let reloaded = WorkspaceState::reload(bundle(&["T-3", "T-2", "T-1"]), Some(&previous)).unwrap();
    assert_eq!(reloaded.selected, Some(2));
    assert_eq!(reloaded.compare, Some(1));
    assert!(reloaded.legacy_expanded);
    assert_eq!(reloaded.filters, previous.filters);
    assert_eq!(reloaded.metrics_sort, previous.metrics_sort);
    assert_eq!(reloaded.est_sort, previous.est_sort);
    assert_eq!(reloaded.matches, [false, false, true]);
    assert_eq!(reloaded.matched_count, 1);
}

#[test]
fn removing_selected_tickets_clears_selection_and_quarantine_expansion() {
    let mut previous = WorkspaceState::reload(bundle(&["T-1", "T-2"]), None).unwrap();
    previous.selected = Some(0);
    previous.compare = Some(1);
    previous.legacy_expanded = true;
    let reloaded = WorkspaceState::reload(bundle(&["T-3"]), Some(&previous)).unwrap();
    assert_eq!(reloaded.selected, None);
    assert_eq!(reloaded.compare, None);
    assert!(!reloaded.legacy_expanded);
}

#[test]
fn corpus_refusal_is_preserved_even_when_the_other_inputs_are_available() {
    let mut input = bundle(&[]);
    let expected = LoadError {
        file: "broken.toml".into(),
        error: "verbatim parse refusal".into(),
    };
    input.corpus = Err(expected.clone());
    match WorkspaceState::reload(input, None) {
        Err(actual) => assert_eq!(actual, expected),
        Ok(_) => panic!("a refused corpus must never become a partial board"),
    }
}
