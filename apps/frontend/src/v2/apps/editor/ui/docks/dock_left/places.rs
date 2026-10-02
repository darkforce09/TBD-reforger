//! Places for the left editor dock.

use super::*;

/// beside the Editor Layers tree, which is this dock's Entities equivalent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    Layers,
    Places,
}

/// `map_engine_core::world::LocationLabel` so the list/filter logic compiles (and is tested) on the
/// native build, where map-engine-core's `world` feature is off.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedPlace {
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// The source row's `kind` (`"town"`, `"village"`, …), empty when the source omits it.
    pub kind: String,
}

/// One predicate for both lists so the bookmark half and the location half can never disagree about
/// what the filter box means.
///
/// `to_lowercase().contains()`. The plain behaviour this function was written for is UNCHANGED (an
/// empty query matches everything; a literal is a case-insensitive substring — those are exactly
/// `SearchPattern::All` and `SearchPattern::Plain` against the `Label` field), and `*`, `?` and
/// `/…/` now work here too. The point is not the extra patterns: it is that the layers tree, the
/// bookmarks list, the locations index and the document search below cannot drift into four ideas of
/// what the box means. A bookmark and a location have no class name and no faction, so `class:` and
/// `mod:` match nothing against them — the honest answer, since those rows carry no such datum.
#[must_use]
pub fn matches_query(name: &str, query: &str) -> bool {
    query_hits(query, name, "", "")
}

/// tab uses. Returns the whole tree unchanged for an empty/blank query (the common case — the filter
/// is off by default), so this is a clone, not a rebuild.
///
/// **A tree filter is not a list filter, and the difference is where the honesty lives.** Two rules:
///
///   * A node whose OWN label matches keeps its **entire subtree**. You searched for a folder because
///     you want what is inside it; hiding its non-matching children would answer a question nobody
///     asked and would make a folder look empty when it is not.
///   * A node that matches only because a DESCENDANT does is kept as **structure**, with just the
///     matching paths under it. Dropping it instead would orphan the hit — the row would appear at
///     the wrong depth, under the wrong parent, in a tree whose whole job is to show containment.
///
/// A node that neither matches nor contains a match is dropped. Pure and native-tested; the view is a
/// thin `Effect` over it.
#[must_use]
pub fn filter_outliner(nodes: &[OutlinerNode], query: &str) -> Vec<OutlinerNode> {
    if query.trim().is_empty() {
        return nodes.to_vec();
    }
    nodes
        .iter()
        .filter_map(|n| keep_matching(n, query))
        .collect()
}

/// [`filter_outliner`]'s recursion: `Some(pruned copy)` when this node survives, `None` when neither
/// it nor anything beneath it matches.
fn keep_matching(node: &OutlinerNode, query: &str) -> Option<OutlinerNode> {
    if matches_query(&node.label, query) {
        return Some(node.clone());
    }
    let kids: Vec<OutlinerNode> = node
        .children
        .iter()
        .filter_map(|c| keep_matching(c, query))
        .collect();
    if kids.is_empty() {
        return None;
    }
    let mut out = node.clone();
    out.children = kids;
    Some(out)
}

/// (`Unfiled`/`Faction`/`Squad`/`Slot`/`Comment` kinds are never drop targets, so only `Folder` nodes
/// answer). `None` when the id is absent — a stale active pointer, exactly the case `ensure_layer`
/// clears before falling back.
#[must_use]
pub fn find_layer_label(nodes: &[OutlinerNode], id: &str) -> Option<String> {
    for n in nodes {
        if n.kind == crate::v2::apps::editor::ui::outliner::outliner::NodeKind::Folder && n.id == id
        {
            return Some(n.label.clone());
        }
        if let Some(found) = find_layer_label(&n.children, id) {
            return Some(found);
        }
    }
    None
}

/// fallback destination. Skips the virtual `Unfiled` root and any ORBAT headers, which are not doc
/// layers and never receive a placement. `None` only when the doc has no layer at all (the empty
/// mission — `ensure_layer` mints `DEFAULT_LAYER` in that case, so the strip below says "a new layer").
#[must_use]
pub fn first_folder_label(nodes: &[OutlinerNode]) -> Option<String> {
    nodes
        .iter()
        .find(|n| n.kind == crate::v2::apps::editor::ui::outliner::outliner::NodeKind::Folder)
        .map(|n| n.label.clone())
}

#[must_use]
/// Return named locations whose labels match the search query.
pub fn filter_places(places: &[NamedPlace], query: &str) -> Vec<NamedPlace> {
    places
        .iter()
        .filter(|p| matches_query(&p.name, query))
        .cloned()
        .collect()
}

/// by eye, so it is sorted by NAME, not by the source file's order or by importance (importance
/// drives the map's declutter, not a list a human reads).
pub fn sort_places(places: &mut [NamedPlace]) {
    places.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
}
