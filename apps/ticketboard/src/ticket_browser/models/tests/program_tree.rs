use super::*;
use crate::test_support::{corpus_of, index_of, program, work};

fn ids_of(corpus: &Corpus, nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| corpus.tickets[n.index].ticket.id().to_owned())
        .collect()
}

fn row_ids(corpus: &Corpus, rows: &[FlatRow]) -> Vec<(String, u16, bool)> {
    rows.iter()
        .map(|r| {
            (
                corpus.tickets[r.index].ticket.id().to_owned(),
                r.depth,
                r.dimmed,
            )
        })
        .collect()
}

/// (program) → (program child-of-program, dotted) → (work,
/// explicit parent) — plus a plain root work ticket.
fn nested_corpus() -> Corpus {
    corpus_of(vec![
        work("T-1", "status = \"idea\"", ""),
        program("T-9", "status = \"queued\"\norder = 10", &["T-9.1"]),
        program("T-9.1", "status = \"queued\"\norder = 10", &["T-9.1.1"]),
        work("T-9.1.1", "status = \"idea\"", "parent = \"T-9.1\"\n"),
    ])
}

#[test]
fn nests_program_child_of_program() {
    let corpus = nested_corpus();
    let tree = TreeModel::build(&corpus, &index_of(&corpus));
    // (order, id): ordered (order 10) roots before unordered (idea).
    assert_eq!(ids_of(&corpus, &tree.roots), vec!["T-9", "T-1"]);
    let t9 = &tree.roots[0];
    assert_eq!(ids_of(&corpus, &t9.children), vec!["T-9.1"]);
    let t91 = &t9.children[0];
    assert_eq!(ids_of(&corpus, &t91.children), vec!["T-9.1.1"]);
    assert!(t91.children[0].children.is_empty());
}

#[test]
fn explicit_parent_beats_dotted_prefix() {
    let corpus = corpus_of(vec![
        program("T-5", "status = \"idea\"", &["T-5.1"]),
        program("T-7", "status = \"idea\"", &["T-5.1"]),
        // Dotted id says; the explicit field says and wins.
        work("T-5.1", "status = \"idea\"", "parent = \"T-7\"\n"),
    ]);
    let tree = TreeModel::build(&corpus, &index_of(&corpus));
    assert_eq!(ids_of(&corpus, &tree.roots), vec!["T-5", "T-7"]);
    assert!(tree.roots[0].children.is_empty());
    assert_eq!(ids_of(&corpus, &tree.roots[1].children), vec!["T-5.1"]);
}

#[test]
fn orphan_dotted_child_roots() {
    // has no file: cannot nest, so it roots (display-only).
    let corpus = corpus_of(vec![work("T-8.1", "status = \"idea\"", "")]);
    let tree = TreeModel::build(&corpus, &index_of(&corpus));
    assert_eq!(ids_of(&corpus, &tree.roots), vec!["T-8.1"]);
}

#[test]
fn siblings_sort_by_order_then_numeric_id() {
    let corpus = corpus_of(vec![
        program("T-2", "status = \"queued\"\norder = 5", &["T-2.1"]),
        work("T-2.2", "status = \"queued\"\norder = 10", ""),
        work("T-2.1", "status = \"queued\"\norder = 20", ""),
        work("T-2.10", "status = \"queued\"\norder = 20", ""),
    ]);
    let tree = TreeModel::build(&corpus, &index_of(&corpus));
    // order 10 first; the order-20 tie breaks numerically (< ).
    assert_eq!(
        ids_of(&corpus, &tree.roots[0].children),
        vec!["T-2.2", "T-2.1", "T-2.10"]
    );
}

#[test]
fn parent_cycle_rescued_as_root() {
    let corpus = corpus_of(vec![
        work("T-3", "status = \"idea\"", "parent = \"T-4\"\n"),
        work("T-4", "status = \"idea\"", "parent = \"T-3\"\n"),
    ]);
    let tree = TreeModel::build(&corpus, &index_of(&corpus));
    // Neither vanishes: the cycle surfaces as one rescued root with the other
    // nested under it.
    assert_eq!(ids_of(&corpus, &tree.roots), vec!["T-3"]);
    assert_eq!(ids_of(&corpus, &tree.roots[0].children), vec!["T-4"]);
}

#[test]
fn flatten_collapsed_shows_roots_only() {
    let corpus = nested_corpus();
    let tree = TreeModel::build(&corpus, &index_of(&corpus));
    let expanded = vec![false; corpus.tickets.len()];
    let rows = flatten(&tree, &expanded, None);
    assert_eq!(
        row_ids(&corpus, &rows),
        vec![("T-9".to_string(), 0, false), ("T-1".to_string(), 0, false)]
    );
    assert!(rows[0].has_children);
    assert!(!rows[0].expanded);
    assert!(!rows[1].has_children);
}

#[test]
fn flatten_manual_expansion_descends() {
    let corpus = nested_corpus();
    let ids = index_of(&corpus);
    let tree = TreeModel::build(&corpus, &ids);
    let mut expanded = vec![false; corpus.tickets.len()];
    expanded[ids["T-9"]] = true;
    let rows = flatten(&tree, &expanded, None);
    assert_eq!(
        row_ids(&corpus, &rows),
        vec![
            ("T-9".to_string(), 0, false),
            ("T-9.1".to_string(), 1, false),
            ("T-1".to_string(), 0, false),
        ]
    );
}

#[test]
fn flatten_filter_force_expands_and_dims_ancestors() {
    let corpus = nested_corpus();
    let ids = index_of(&corpus);
    let tree = TreeModel::build(&corpus, &ids);
    let expanded = vec![false; corpus.tickets.len()];
    // Only the grandchild matches.
    let mut matches = vec![false; corpus.tickets.len()];
    matches[ids["T-9.1.1"]] = true;
    let rows = flatten(&tree, &expanded, Some(&matches));
    assert_eq!(
        row_ids(&corpus, &rows),
        vec![
            ("T-9".to_string(), 0, true),
            ("T-9.1".to_string(), 1, true),
            ("T-9.1.1".to_string(), 2, false),
        ]
    );
    // Non-matching subtree is gone entirely; ancestors are force-open.
    assert!(rows[0].expanded && rows[1].expanded);
}
