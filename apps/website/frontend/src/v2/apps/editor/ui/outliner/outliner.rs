//! The left dock's Editor Layers and ORBAT outliner trees.
//!
//! Each editor layer contains child folders followed by placed slots. Unfiled slots appear under
//! a virtual root whose id is never written to the document or used as an active layer. Real
//! folders keep their authored entity order; unfiled children sort by id for stable display.
//! The tree operates on plain rows so its construction and ordering work in native tests.
#![allow(dead_code)]

use std::collections::HashSet;
pub use website_map_engine::data::store::operations::rows::CommentRow;
pub use website_map_engine::data::store::operations::rows::FactionRow;
pub use website_map_engine::data::store::operations::rows::LayerRow;
pub use website_map_engine::data::store::operations::rows::SlotRow;
pub use website_map_engine::data::store::operations::rows::SquadRow;

/// The virtual root's id. Not a doc id — see the module docs.
pub const UNFILED_ID: &str = "__unfiled";
/// The id of the folder a place mints when no folder is active. A doc id, unlike [`UNFILED_ID`],
/// and the same id a server hydrate files an unlayered slot under, so a freshly authored mission
/// and a re-hydrated one agree on where an entity with no folder of its own lives.
pub const DEFAULT_LAYER_ID: &str = "layer-1";
/// That folder's name in the tree. Naming is the dock's vocabulary rather than the document's law,
/// which is why it is handed to the document at mint time instead of being assumed there.
pub const DEFAULT_LAYER_NAME: &str = "Layer 1";
/// above this many flattened rows a tree renders windowed (React `VIRTUAL_SLOT_THRESHOLD`,
/// proven @ ~367k). Below it, the eager recursive render is cheaper and keeps native scroll simple.
pub const VIRTUAL_SLOT_THRESHOLD: usize = 50;
/// React's `label: s.role || 'Unit'` fallback (`EditorLayersSection.tsx:66`).
const SLOT_FALLBACK_LABEL: &str = "Unit";
/// an untitled comment's row label (the `SLOT_FALLBACK_LABEL` idiom: a row must always be
/// clickable, and a blank title would render a zero-width row you cannot select to fix).
pub const COMMENT_FALLBACK_LABEL: &str = "Comment";

/// What a row represents — the view needs this to route a click (folder → active layer, slot →
/// selection) and to pick a glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A real `editorLayers` folder — its id IS a doc id.
    Folder,
    /// The virtual "Unfiled" root — [`UNFILED_ID`], never a doc id.
    Unfiled,
    Slot,
    /// an ORBAT faction group header (id is the faction doc id).
    Faction,
    /// an ORBAT squad group header (id is the squad doc id).
    Squad,
    /// an editor-only COMMENT (`PLACE-COMMENT-001`). Its id is a `commentsById` key, never a
    /// slot id: a row of this kind must never be routed into `select_slot` / `open_attributes` (a
    /// comment is in no selection lane and has no Attributes modal), and it never compiles.
    Comment,
}

/// One row of the Outliner tree: what it is, what it says, what it contains, and the glyph state
/// its toggles render from.
///
/// Built fresh from the document every time the tree is rebuilt, so nothing here is authority —
/// the document is. In particular the resolved-visibility bit is derived at build time from this
/// node's ancestors and is never written back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlinerNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub children: Vec<OutlinerNode>,
    /// true when this slot is `squad.leaderSlotId` (ORBAT SL badge; never from `tag`).
    pub is_leader: bool,
    /// the eye/lock glyph state on a Folder row: this layer's OWN `hidden` flag (the eye
    /// toggle fills/outlines from this bit, and flips only this layer). Always `false` on non-folder
    /// kinds. Distinct from [`Self::hidden_effective`] so a folder shows its own state on the toggle
    /// while a hidden PARENT still dims the child rows.
    pub hidden: bool,
    /// this layer's OWN `locked` flag (Folder rows only; the lock toggle reads this).
    pub locked: bool,
    /// RESOLVED visibility: this node (folder or slot) sits under a hidden layer/ancestor,
    /// so the row renders dimmed. Mirrors [`crate::doc`]'s materialize filter — a slot with this set
    /// is exactly one the render SoA dropped. Resolved at build time; never written into the doc.
    pub hidden_effective: bool,
    /// RESOLVED lock: this node is under a locked layer/ancestor (drives the row's lock
    /// adornment + a disabled affordance hint). Its slots refuse a move at the store level.
    pub locked_effective: bool,
    /// Hover text for comment bodies that do not fit in the row label.
    pub tooltip: String,
}

fn slot_node(s: &SlotRow) -> OutlinerNode {
    slot_node_leader(s, false)
}

fn slot_node_leader(s: &SlotRow, is_leader: bool) -> OutlinerNode {
    slot_node_full(s, is_leader, false, false)
}

/// slot node carrying the resolved hidden/locked state inherited from its layer chain.
fn slot_node_full(
    s: &SlotRow,
    is_leader: bool,
    hidden_effective: bool,
    locked_effective: bool,
) -> OutlinerNode {
    OutlinerNode {
        id: s.id.clone(),
        label: if s.role.is_empty() {
            SLOT_FALLBACK_LABEL.to_string()
        } else {
            s.role.clone()
        },
        kind: NodeKind::Slot,
        children: Vec::new(),
        is_leader,
        hidden: false,
        locked: false,
        hidden_effective,
        locked_effective,
        tooltip: String::new(),
    }
}

/// a comment leaf. Carries no hidden/locked state: those are per-LAYER transform/visibility
/// contracts on mission geometry, and a comment is neither hidden from a render (it is not in the
/// render SoA at all — it never reaches `materialize`) nor transform-lockable (see
/// `MissionDocCore::set_comment_position` for why a locked layer does not freeze its own note).
/// Inheriting the dim/lock adornments would advertise a refusal that does not exist.
fn comment_node(c: &CommentRow) -> OutlinerNode {
    OutlinerNode {
        id: c.id.clone(),
        label: if c.title.is_empty() {
            COMMENT_FALLBACK_LABEL.to_string()
        } else {
            c.title.clone()
        },
        kind: NodeKind::Comment,
        children: Vec::new(),
        is_leader: false,
        hidden: false,
        locked: false,
        hidden_effective: false,
        locked_effective: false,
        tooltip: c.tooltip.clone(),
    }
}

/// Build the outliner: the "Unfiled" pseudo-root (when any slot is filed nowhere) followed by the
/// real root layers. See the module docs for the divergences and the ordering rule.
///
/// the comment-free form, kept as its own entry point so every caller that has no comments
/// to show (and every test that predates them) reads unchanged. The live editor dock calls
/// [`build_outliner_with_comments`].
#[must_use]
pub fn build_outliner(layers: &[LayerRow], slots: &[SlotRow]) -> Vec<OutlinerNode> {
    build_outliner_with_comments(layers, slots, &[])
}

/// the outliner including editor-only COMMENT rows (`PLACE-COMMENT-001`).
///
/// A comment is placed by the SAME rule a slot is, because it is filed by the same mechanism: it
/// belongs to the first layer whose `entityIds` lists its id, and one listed nowhere lands in the
/// "Unfiled" pseudo-root. There is no parallel comment-filing structure to drift — see
/// `MissionDocCore::move_comment_to_layer`, which literally delegates to `move_slot_to_layer`.
///
/// Inside a folder the `entityIds` sequence is authoritative for BOTH kinds (React parity for slots,
/// and the only order a comment has), so a comment sits exactly where the operator dropped it rather
/// than in a segregated block. In the Unfiled root, slots come first and then comments, each sorted
/// by id — Unfiled has no authored order at all (see the module docs), so a stable, kind-grouped
/// order is the readable choice and the gate-exact one.
#[must_use]
pub fn build_outliner_with_comments(
    layers: &[LayerRow],
    slots: &[SlotRow],
    comments: &[CommentRow],
) -> Vec<OutlinerNode> {
    let mut out: Vec<OutlinerNode> = Vec::new();

    // Reverse index, matching `MissionDocCore::materialize` (`store.rs:206-221`): a slot belongs to
    // the FIRST layer whose `entityIds` lists it; one in none is unfiled. the same index
    // answers the same question for a comment id, because they share the array.
    let filed: HashSet<&str> = layers
        .iter()
        .flat_map(|l| l.entity_ids.iter().map(String::as_str))
        .collect();

    let mut unfiled: Vec<&SlotRow> = slots
        .iter()
        .filter(|s| !filed.contains(s.id.as_str()))
        .collect();
    unfiled.sort_by(|a, b| a.id.cmp(&b.id)); // deterministic; materialize order is arbitrary
    let mut unfiled_comments: Vec<&CommentRow> = comments
        .iter()
        .filter(|c| !filed.contains(c.id.as_str()))
        .collect();
    unfiled_comments.sort_by(|a, b| a.id.cmp(&b.id));
    if !unfiled.is_empty() || !unfiled_comments.is_empty() {
        let n = unfiled.len() + unfiled_comments.len();
        let children: Vec<OutlinerNode> = unfiled
            .into_iter()
            .map(slot_node)
            .chain(unfiled_comments.into_iter().map(comment_node))
            .collect();
        out.push(OutlinerNode {
            id: UNFILED_ID.to_string(),
            label: format!("Unfiled ({n})"),
            kind: NodeKind::Unfiled,
            // Unfiled slots are in no layer, so they can inherit neither hidden nor locked.
            children,
            is_leader: false,
            hidden: false,
            locked: false,
            hidden_effective: false,
            locked_effective: false,
            tooltip: String::new(),
        });
    }

    for root in layers.iter().filter(|l| l.parent_id.is_none()) {
        // `seen` guards a malformed `parentId` cycle. The core's `reparent_editor_layer` is
        // cycle-guarded (`store.rs:826`), so this is belt-and-braces — but an unguarded recursion
        // would hang the tab rather than render wrong, which is not a trade worth taking.
        let mut seen = HashSet::new();
        // Roots have no ancestor, so the inherited flags start `false`.
        out.push(build_layer(
            root, layers, slots, comments, false, false, &mut seen,
        ));
    }

    out
}

/// `anc_hidden`/`anc_locked`: whether an ANCESTOR layer is hidden/locked. The layer's own
/// flag ORs into the effective state passed to its children, so hiding/locking a folder covers its
/// whole subtree without ever writing the flag onto a descendant row (resolve-at-build, matching the
/// core's resolve-at-read). A row's own toggle glyph still shows its OWN flag.
fn build_layer<'a>(
    layer: &'a LayerRow,
    layers: &'a [LayerRow],
    slots: &[SlotRow],
    comments: &[CommentRow],
    anc_hidden: bool,
    anc_locked: bool,
    seen: &mut HashSet<&'a str>,
) -> OutlinerNode {
    // Effective state for THIS folder and everything under it = ancestor state OR its own flag.
    let hidden_effective = anc_hidden || layer.hidden;
    let locked_effective = anc_locked || layer.locked;
    let mut children: Vec<OutlinerNode> = Vec::new();
    if seen.insert(layer.id.as_str()) {
        // Child folders first, then this folder's slots — React's `[...childFolders, ...entityNodes]`.
        for child in layers
            .iter()
            .filter(|l| l.parent_id.as_deref() == Some(layer.id.as_str()))
        {
            children.push(build_layer(
                child,
                layers,
                slots,
                comments,
                hidden_effective,
                locked_effective,
                seen,
            ));
        }
        // `entityIds` order (React parity). A dangling id (slot deleted, layer not yet patched) is
        // skipped, mirroring React's `.filter((s): s is Slot => Boolean(s))`. A slot inherits this
        // folder's effective hidden/locked state.
        //
        // an id that is not a slot may be a COMMENT (they share this array by design). Slot
        // is tried first: ids come from disjoint mints, so the order is not a tie-break but a cheap
        // ordering of the common case, and an id in neither map is still skipped as dangling.
        for eid in &layer.entity_ids {
            if let Some(s) = slots.iter().find(|s| &s.id == eid) {
                children.push(slot_node_full(s, false, hidden_effective, locked_effective));
            } else if let Some(c) = comments.iter().find(|c| &c.id == eid) {
                children.push(comment_node(c));
            }
        }
    }

    OutlinerNode {
        id: layer.id.clone(),
        label: layer.name.clone(),
        kind: NodeKind::Folder,
        children,
        is_leader: false,
        hidden: layer.hidden,
        locked: layer.locked,
        hidden_effective,
        locked_effective,
        tooltip: String::new(),
    }
}

/* ───────────────────────────── ORBAT tree ───────────────────────────── */

mod flatten;
/// Build the ORBAT browse tree: faction → squad → slot, in doc order (`squadIds` / `slotIds`).
/// A dangling id (deleted slot/squad, container not yet patched) is skipped — the `build_outliner`
/// filter idiom. Empty until the first placed slot mints a default faction+squad.
#[must_use]
mod orbat;

#[cfg(target_arch = "wasm32")]
pub use flatten::{create_layer, delete_layer, ensure_active_layer, set_active_layer};
pub use flatten::{flatten, flatten_visible, FlatRow};
pub use orbat::{
    build_orbat, filter_orbat_squads_by_side_key, ORBAT_MANAGER_DIALOG_CLASS, ORBAT_MANAGER_EMPTY,
};

#[cfg(test)]
#[path = "tests/outliner_model/outliner_hierarchy_visibility_and_comments.rs"]
mod outliner_hierarchy_visibility_and_comments;
