//! Program-tree projection (T-915.2 §UI shape) — pure, unit-tested, no egui types.
//!
//! Top-level tickets (no resolvable parent) with children nested recursively —
//! children can themselves be programs. Parent resolution per ticket: the explicit
//! work-ticket `parent` field when that id exists in the corpus, else the dotted-id
//! prefix (`T-915.2` → `T-915`) when THAT exists, else the ticket roots. Siblings
//! sort by `(order, id)` — the board's card order. Parent cycles (bad explicit
//! `parent` fields) fall back to roots instead of vanishing: display-only, no
//! judgment.

use std::collections::HashMap;

use crate::board;
use crate::corpus::Corpus;

pub struct Node {
    pub index: usize,
    pub children: Vec<Node>,
}

pub struct TreeModel {
    pub roots: Vec<Node>,
    /// Truncated titles by corpus index — precomputed so the paint path never
    /// formats strings.
    pub titles: Vec<String>,
}

/// One paint row of the flattened tree (the virtualized `show_rows` surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlatRow {
    pub index: usize,
    pub depth: u16,
    pub has_children: bool,
    pub expanded: bool,
    /// Shown only as an ancestor of a filter match — painted dimmed.
    pub dimmed: bool,
}

const TITLE_MAX_CHARS: usize = 56;

/// `(order, numeric id)` — identical to the board's card sort.
fn sort_key(corpus: &Corpus, index: usize) -> (i64, Vec<u64>, String) {
    let t = &corpus.tickets[index].ticket;
    let (segments, raw) = board::id_sort_key(t.id());
    (t.status().order().unwrap_or(i64::MAX), segments, raw)
}

/// Explicit `parent` field first, dotted-id prefix second — the first that resolves
/// to a ticket in the corpus. Self-parents count as unresolved.
fn parent_index(corpus: &Corpus, ids: &HashMap<String, usize>, index: usize) -> Option<usize> {
    let t = &corpus.tickets[index].ticket;
    let explicit = board::view(t).parent.and_then(|p| ids.get(p));
    let dotted = t
        .id()
        .rsplit_once('.')
        .and_then(|(prefix, _)| ids.get(prefix));
    explicit.or(dotted).copied().filter(|&p| p != index)
}

impl TreeModel {
    pub fn build(corpus: &Corpus, ids: &HashMap<String, usize>) -> Self {
        let n = corpus.tickets.len();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut roots: Vec<usize> = Vec::new();
        for i in 0..n {
            match parent_index(corpus, ids, i) {
                Some(p) => kids[p].push(i),
                None => roots.push(i),
            }
        }
        for list in &mut kids {
            list.sort_by_key(|&i| sort_key(corpus, i));
        }
        roots.sort_by_key(|&i| sort_key(corpus, i));

        let mut visited = vec![false; n];
        let mut root_nodes: Vec<Node> = roots
            .iter()
            .map(|&i| build_node(i, &kids, &mut visited))
            .collect();
        // Cycle rescue: a parent loop is unreachable from every root — surface its
        // members as extra roots rather than dropping tickets from the tree.
        let mut leftover: Vec<usize> = (0..n).filter(|&i| !visited[i]).collect();
        leftover.sort_by_key(|&i| sort_key(corpus, i));
        for i in leftover {
            if !visited[i] {
                root_nodes.push(build_node(i, &kids, &mut visited));
            }
        }

        let titles = corpus
            .tickets
            .iter()
            .map(|t| board::truncate_chars(board::title_of(&t.ticket), TITLE_MAX_CHARS))
            .collect();
        Self {
            roots: root_nodes,
            titles,
        }
    }
}

fn build_node(index: usize, kids: &[Vec<usize>], visited: &mut [bool]) -> Node {
    visited[index] = true;
    let mut children = Vec::with_capacity(kids[index].len());
    for &child in &kids[index] {
        if !visited[child] {
            children.push(build_node(child, kids, visited));
        }
    }
    Node { index, children }
}

/// Flatten the visible tree for virtualized painting.
///
/// `filter == None`: manual expansion only. `filter == Some(matches)`: a node is
/// visible when it matches or has a matching descendant; paths to matches are
/// force-expanded (else matches would hide under collapsed roots) and non-matching
/// ancestors come back `dimmed`.
pub fn flatten(model: &TreeModel, expanded: &[bool], filter: Option<&[bool]>) -> Vec<FlatRow> {
    let mut rows = Vec::new();
    for node in &model.roots {
        flatten_node(node, 0, expanded, filter, &mut rows);
    }
    rows
}

fn subtree_has_match(node: &Node, matches: &[bool]) -> bool {
    matches[node.index] || node.children.iter().any(|c| subtree_has_match(c, matches))
}

fn flatten_node(
    node: &Node,
    depth: u16,
    expanded: &[bool],
    filter: Option<&[bool]>,
    rows: &mut Vec<FlatRow>,
) {
    let (visible, dimmed, force_open) = match filter {
        None => (true, false, false),
        Some(matches) => {
            let self_match = matches[node.index];
            let descendant_match = node.children.iter().any(|c| subtree_has_match(c, matches));
            (
                self_match || descendant_match,
                !self_match,
                descendant_match,
            )
        }
    };
    if !visible {
        return;
    }
    let open = force_open || (filter.is_none() && expanded[node.index]);
    rows.push(FlatRow {
        index: node.index,
        depth,
        has_children: !node.children.is_empty(),
        expanded: open,
        dimmed,
    });
    if open {
        for child in &node.children {
            flatten_node(child, depth + 1, expanded, filter, rows);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{corpus_of, index_of, program, work};

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

    /// T-9 (program) → T-9.1 (program child-of-program, dotted) → T-9.1.1 (work,
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
        // (order, id): ordered T-9 (order 10) roots before unordered T-1 (idea).
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
            // Dotted id says T-5; the explicit field says T-7 and wins.
            work("T-5.1", "status = \"idea\"", "parent = \"T-7\"\n"),
        ]);
        let tree = TreeModel::build(&corpus, &index_of(&corpus));
        assert_eq!(ids_of(&corpus, &tree.roots), vec!["T-5", "T-7"]);
        assert!(tree.roots[0].children.is_empty());
        assert_eq!(ids_of(&corpus, &tree.roots[1].children), vec!["T-5.1"]);
    }

    #[test]
    fn orphan_dotted_child_roots() {
        // T-8 has no file: T-8.1 cannot nest, so it roots (display-only).
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
        // order 10 first; the order-20 tie breaks numerically (T-2.1 < T-2.10).
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
        // Non-matching subtree (T-1) is gone entirely; ancestors are force-open.
        assert!(rows[0].expanded && rows[1].expanded);
    }
}
