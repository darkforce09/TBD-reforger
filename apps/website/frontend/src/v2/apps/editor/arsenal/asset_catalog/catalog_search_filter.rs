//! Asset catalog catalog search filter behavior.

use super::*;

/// Keeps folders and leaves matching a catalog query.
#[must_use]
pub fn filter_catalog(nodes: &[CatalogNode], query: &str) -> Vec<CatalogNode> {
    let q = parse_search_query(query);
    match q.pattern {
        SearchPattern::All => return nodes.to_vec(),
        SearchPattern::Pending | SearchPattern::Invalid => return Vec::new(),
        _ => {}
    }
    let p = &q.pattern;
    match q.field {
        SearchField::Label => {
            fn keep(node: &CatalogNode, p: &SearchPattern) -> Option<CatalogNode> {
                if p.hits(&node.label, false) {
                    return Some(node.clone()); // self-match → full subtree
                }
                let children: Vec<CatalogNode> =
                    node.children.iter().filter_map(|c| keep(c, p)).collect();
                if children.is_empty() {
                    return None;
                }
                let mut out = node.clone();
                out.children = children;
                Some(out)
            }
            nodes.iter().filter_map(|n| keep(n, p)).collect()
        }
        SearchField::ClassName => {
            fn keep(node: &CatalogNode, p: &SearchPattern) -> Option<CatalogNode> {
                if node.payload.is_some() {
                    let id = &node.id;
                    return (p.hits(id, true) || p.hits(classname_tail(id), true))
                        .then(|| node.clone());
                }
                let children: Vec<CatalogNode> =
                    node.children.iter().filter_map(|c| keep(c, p)).collect();
                if children.is_empty() {
                    return None;
                }
                let mut out = node.clone();
                out.children = children;
                Some(out)
            }
            nodes.iter().filter_map(|n| keep(n, p)).collect()
        }
        SearchField::Mod => nodes
            .iter()
            .filter(|n| p.hits(&n.label, true))
            .cloned()
            .collect(),
    }
}
