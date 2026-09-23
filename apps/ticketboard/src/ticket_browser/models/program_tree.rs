//! Program-tree projection with numeric ticket ordering and cycle protection.
//!
//! Explicit parents take priority, then existing dotted-ID ancestors. Filtering reveals
//! matching descendant paths while retaining their parent context.

use std::collections::HashMap;

use crate::ticket_registry::models::corpus::Corpus;
use crate::ticket_registry::models::projection as board;

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
#[path = "tests/program_tree.rs"]
mod tests;
