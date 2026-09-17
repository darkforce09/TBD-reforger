//! T-159.22 — the left dock's **Editor Layers** outliner tree.
//!
//! Ports React's `buildTree` (`layout/LeftOutliner/EditorLayersSection.tsx:51-81`): each editor
//! layer is a folder holding its child folders **then** its placed slots; layers nest via
//! `parentId` (`None` = root); a slot's label is its `role`, or `"Unit"` when empty.
//!
//! ## The "Unfiled" pseudo-root (a deliberate divergence from React)
//!
//! React cannot have a slot outside a folder — its `addSlot` always runs `ensureDefaultLayer` — so
//! `buildTree` renders **only** layers and their `entityIds`. The Leptos editor's seed does not go
//! through `add_slot` at all: `MissionDocCore::seed_random` (`store.rs:348`) writes the `slots` map
//! directly, creating **no layers and no squads**. A literal port would therefore render an empty
//! dock while the toolbelt's OBJ read `8`.
//!
//! Creating a default layer at boot is not an option either: `smoke_save_export_editor` asserts
//! `editor.editorLayers.length === 0`. So unfiled slots get a virtual root instead, and the default
//! layer is minted lazily on the first place (LOCAL origin ⇒ undoable), mirroring React's
//! `ensureDefaultLayer`-inside-`addSlot`. [`UNFILED_ID`] is not a doc id — the view must never pass
//! it to `move_slot_to_layer` or make it the active layer.
//!
//! ## Ordering
//!
//! Real folders keep `entityIds` order (React parity — insertion order). **Unfiled children sort by
//! slot id**, because their only other source of order would be `materialize()`'s row order, which
//! is arbitrary (`yrs` map iteration). Sorting makes the tree stable for the operator and exact for
//! the gate.
//!
//! Pure + native-testable on purpose: this module owns plain [`LayerRow`] / [`SlotRow`] instead of
//! importing `SlotSoa`, because `map-engine-core` is a **wasm32-only** dependency. The caller
//! (`mission_editor`) adapts the doc's `small_maps_json()` + `materialize()` into these rows.
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
/// T-169 — above this many flattened rows a tree renders windowed (React `VIRTUAL_SLOT_THRESHOLD`,
/// proven @ ~367k). Below it, the eager recursive render is cheaper and keeps native scroll simple.
pub const VIRTUAL_SLOT_THRESHOLD: usize = 50;
/// React's `label: s.role || 'Unit'` fallback (`EditorLayersSection.tsx:66`).
const SLOT_FALLBACK_LABEL: &str = "Unit";
/// T-651 — an untitled comment's row label (the `SLOT_FALLBACK_LABEL` idiom: a row must always be
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
    /// T-168 — an ORBAT faction group header (id is the faction doc id).
    Faction,
    /// T-168 — an ORBAT squad group header (id is the squad doc id).
    Squad,
    /// T-651 — an editor-only COMMENT (`PLACE-COMMENT-001`). Its id is a `commentsById` key, never a
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
    /// T-180.6 — true when this slot is `squad.leaderSlotId` (ORBAT SL badge; never from `tag`).
    pub is_leader: bool,
    /// T-665 — the eye/lock glyph state on a Folder row: this layer's OWN `hidden` flag (the eye
    /// toggle fills/outlines from this bit, and flips only this layer). Always `false` on non-folder
    /// kinds. Distinct from [`Self::hidden_effective`] so a folder shows its own state on the toggle
    /// while a hidden PARENT still dims the child rows.
    pub hidden: bool,
    /// T-665 — this layer's OWN `locked` flag (Folder rows only; the lock toggle reads this).
    pub locked: bool,
    /// T-665 — RESOLVED visibility: this node (folder or slot) sits under a hidden layer/ancestor,
    /// so the row renders dimmed. Mirrors [`crate::doc`]'s materialize filter — a slot with this set
    /// is exactly one the render SoA dropped. Resolved at build time; never written into the doc.
    pub hidden_effective: bool,
    /// T-665 — RESOLVED lock: this node is under a locked layer/ancestor (drives the row's lock
    /// adornment + a disabled affordance hint). Its slots refuse a move at the store level.
    pub locked_effective: bool,
    /// T-651 — hover text. Non-empty only on [`NodeKind::Comment`] rows, where it carries
    /// ATTR-FIELD-CMT-TOOLTIP. A comment's whole point is a body too long for a label (FNF v3's
    /// tutorial ran seven paragraphs), so the tree has to carry it or the annotation is unreadable
    /// without a second dialog this ticket does not ship.
    pub tooltip: String,
}

fn slot_node(s: &SlotRow) -> OutlinerNode {
    slot_node_leader(s, false)
}

fn slot_node_leader(s: &SlotRow, is_leader: bool) -> OutlinerNode {
    slot_node_full(s, is_leader, false, false)
}

/// T-665 — slot node carrying the resolved hidden/locked state inherited from its layer chain.
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

/// T-651 — a comment leaf. Carries no hidden/locked state: those are per-LAYER transform/visibility
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
/// T-651 — the comment-free form, kept as its own entry point so every caller that has no comments
/// to show (and every test that predates them) reads unchanged. The live editor dock calls
/// [`build_outliner_with_comments`].
#[must_use]
pub fn build_outliner(layers: &[LayerRow], slots: &[SlotRow]) -> Vec<OutlinerNode> {
    build_outliner_with_comments(layers, slots, &[])
}

/// T-651 — the outliner including editor-only COMMENT rows (`PLACE-COMMENT-001`).
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
    // the FIRST layer whose `entityIds` lists it; one in none is unfiled. T-651 — the same index
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

/// T-665 — `anc_hidden`/`anc_locked`: whether an ANCESTOR layer is hidden/locked. The layer's own
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
        // T-651 — an id that is not a slot may be a COMMENT (they share this array by design). Slot
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

/* ───────────────────────────── T-168 — ORBAT tree ───────────────────────────── */

/// Build the ORBAT browse tree: faction → squad → slot, in doc order (`squadIds` / `slotIds`).
/// A dangling id (deleted slot/squad, container not yet patched) is skipped — the `build_outliner`
/// filter idiom. Empty until the first placed slot mints a default faction+squad (T-168 place-mint).
#[must_use]
pub fn build_orbat(
    factions: &[FactionRow],
    squads: &[SquadRow],
    slots: &[SlotRow],
) -> Vec<OutlinerNode> {
    let squad_by_id = |id: &str| squads.iter().find(|s| s.id == id);
    let slot_by_id = |id: &str| slots.iter().find(|s| s.id == id);

    let mut out: Vec<OutlinerNode> = Vec::new();
    // Deterministic faction order (doc map iteration is arbitrary).
    let mut ordered: Vec<&FactionRow> = factions.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    for f in ordered {
        let squad_nodes: Vec<OutlinerNode> = f
            .squad_ids
            .iter()
            .filter_map(|sid| squad_by_id(sid))
            .map(|sq| {
                let slot_children: Vec<OutlinerNode> = sq
                    .slot_ids
                    .iter()
                    .filter_map(|id| slot_by_id(id))
                    .map(|s| {
                        let is_leader = !sq.leader_slot_id.is_empty() && s.id == sq.leader_slot_id;
                        slot_node_leader(s, is_leader)
                    })
                    .collect();
                OutlinerNode {
                    id: sq.id.clone(),
                    label: format!("{} ({})", sq.name, slot_children.len()),
                    kind: NodeKind::Squad,
                    children: slot_children,
                    is_leader: false,
                    // ORBAT tree is squad-scoped, not layer-scoped — flags never apply here.
                    hidden: false,
                    locked: false,
                    hidden_effective: false,
                    locked_effective: false,
                    // ORBAT rows are never comments (comments live in the layer tree only).
                    tooltip: String::new(),
                }
            })
            .collect();
        out.push(OutlinerNode {
            id: f.id.clone(),
            label: f.name.clone(),
            kind: NodeKind::Faction,
            children: squad_nodes,
            is_leader: false,
            hidden: false,
            locked: false,
            hidden_effective: false,
            locked_effective: false,
            tooltip: String::new(),
        });
    }
    out
}

/// T-180.7 — filter ORBAT tree to squads under factions whose [`FactionRow::key`] equals `side_key`.
/// Uses **key**, never name substring (G8). Returns squad nodes only (side tabs replace faction headers).
#[must_use]
pub fn filter_orbat_squads_by_side_key(
    factions: &[FactionRow],
    squads: &[SquadRow],
    slots: &[SlotRow],
    side_key: &str,
) -> Vec<OutlinerNode> {
    let tree = build_orbat(factions, squads, slots);
    let matching_faction_ids: std::collections::HashSet<&str> = factions
        .iter()
        .filter(|f| f.key == side_key)
        .map(|f| f.id.as_str())
        .collect();
    tree.into_iter()
        .filter(|n| matching_faction_ids.contains(n.id.as_str()))
        .flat_map(|f| f.children)
        .collect()
}

/// G1 — near-fullscreen dialog class list (Faction Manager pattern; not `ui::Dialog` / `max-w-xl`).
pub const ORBAT_MANAGER_DIALOG_CLASS: &str = "glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex h-[min(800px,90vh)] w-[min(1100px,95vw)] max-w-6xl -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none";

/// G7 — empty-state when the filtered side has no squads (never Stitch sample strings).
pub const ORBAT_MANAGER_EMPTY: &str = "No squads on this side yet — place a unit or add a squad.";

/* ───────────────────────────── T-169 — flattened rows for windowing ───────────────────────────── */

/// One flattened tree row (pre-order): the node's identity + its nesting depth. The windowed
/// renderer slices a `Vec<FlatRow>` and draws only the visible span (React `flattenOutliner`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatRow {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub depth: usize,
    /// True when the source node has children — drives the chevron + open/closed folder icon
    /// (T-172 B6/B7). A collapsed container still renders its own row; its subtree does not.
    pub has_children: bool,
    /// T-177 A1 — YouTube-style guide continuation, one bool per guide column (`len == depth`):
    /// `ancestors[k]` = "the vertical line at column k continues below this row." For `k < depth-1`
    /// it's an ancestor spine (drawn iff that ancestor has a following sibling); `ancestors[depth-1]`
    /// is this row's own connector, whose bit = `!is_last` (draw the elbow's tail down to the next
    /// sibling, or trim it at the last child). Self-contained per row so the windowed slice needs no
    /// sibling lookup. Roots (depth 0) get `[]` → no guides. See `eden_chrome::guide_spans`.
    pub ancestors: Vec<bool>,
    /// T-178 A4 — owner id per guide column (`len == depth`); `guide_ids[k]` toggles on guide click.
    pub guide_ids: Vec<String>,
    /// T-180.6 — copied from [`OutlinerNode::is_leader`] for the windowed SL badge.
    pub is_leader: bool,
    /// T-665 — this Folder layer's OWN `hidden` flag (drives the eye-toggle glyph state).
    pub hidden: bool,
    /// T-665 — this Folder layer's OWN `locked` flag (drives the lock-toggle glyph state).
    pub locked: bool,
    /// T-665 — RESOLVED hidden (own or inherited): the windowed row renders dimmed when set.
    pub hidden_effective: bool,
    /// T-665 — RESOLVED lock (own or inherited): the windowed row shows the inherited-lock adornment.
    pub locked_effective: bool,
    /// T-651 — copied from [`OutlinerNode::tooltip`] for the windowed comment row's hover text.
    pub tooltip: String,
}

/// Flatten a tree to pre-order rows (parent before its children). Every node becomes exactly one
/// row — the window operates on this flat list, not the nested `OutlinerNode`s.
#[must_use]
pub fn flatten(nodes: &[OutlinerNode]) -> Vec<FlatRow> {
    flatten_visible(nodes, &std::collections::HashSet::new())
}

/// Flatten honoring a collapsed-id set (T-172 B6): a collapsed node emits its own row but none
/// of its descendants. An empty set = the old fully-expanded `flatten`.
#[must_use]
pub fn flatten_visible(
    nodes: &[OutlinerNode],
    collapsed: &std::collections::HashSet<String>,
) -> Vec<FlatRow> {
    let mut out = Vec::new();
    // `prefix` = the parent row's `ancestors` vector (length == parent depth). A child's vector is
    // the parent's + its own `!is_last` bit (T-177 A1); roots (depth 0) draw no guide column, so
    // their own bit is dropped (`ancestors == []`) and it never propagates as a spine — a depth-1
    // row's single column is its OWN elbow, not a root spine.
    // `id_prefix` = ancestor node ids for guide click (T-178 A4); `len == depth`.
    fn walk(
        nodes: &[OutlinerNode],
        depth: usize,
        prefix: &[bool],
        id_prefix: &[String],
        collapsed: &std::collections::HashSet<String>,
        out: &mut Vec<FlatRow>,
    ) {
        let len = nodes.len();
        for (i, n) in nodes.iter().enumerate() {
            let is_last = i + 1 == len;
            let ancestors: Vec<bool> = if depth == 0 {
                Vec::new()
            } else {
                let mut v = Vec::with_capacity(depth);
                v.extend_from_slice(prefix);
                v.push(!is_last);
                v
            };
            let guide_ids = id_prefix.to_vec();
            out.push(FlatRow {
                id: n.id.clone(),
                label: n.label.clone(),
                kind: n.kind,
                depth,
                has_children: !n.children.is_empty(),
                ancestors: ancestors.clone(),
                guide_ids: guide_ids.clone(),
                is_leader: n.is_leader,
                hidden: n.hidden,
                locked: n.locked,
                hidden_effective: n.hidden_effective,
                locked_effective: n.locked_effective,
                tooltip: n.tooltip.clone(),
            });
            if !collapsed.contains(&n.id) {
                let mut child_ids = guide_ids;
                child_ids.push(n.id.clone());
                walk(
                    &n.children,
                    depth + 1,
                    &ancestors,
                    &child_ids,
                    collapsed,
                    out,
                );
            }
        }
    }
    walk(nodes, 0, &[], &[], collapsed, &mut out);
    out
}

/// The folder a new entity is filed under when the tree has one focused, and the gestures that
/// move that focus. Wasm-only because they reach the live mission document; the tree vocabulary
/// above them is pure and native-testable, which is why the two halves share this module rather
/// than a wrapper somewhere else.
#[cfg(target_arch = "wasm32")]
mod active_folder {
    use super::{DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME};
    use crate::v2::apps::editor::bridge::host_state::editor_context::{
        EditorContext, EDITOR_CONTEXT,
    };
    use leptos::prelude::{GetUntracked, Set};
    use website_map_engine::data::store::MissionDocCore;
    use website_map_engine::editing::hosted_commands as engine_ops;

    /// Focus a folder, or clear the focus. A focused folder is the drop target for the next place.
    pub fn set_active_layer(id: Option<String>) {
        EDITOR_CONTEXT.with(|c| {
            if let Some(ctx) = c.borrow().as_ref() {
                ctx.active_layer.set(id);
            }
        });
    }

    /// Resolve the folder a new entity is filed under against one context: the focused folder when
    /// one is set and still live, otherwise the default folder, minted under the LOCAL origin so
    /// the mint is part of the same undoable act as the place it serves. A focus pointing at a
    /// folder the document no longer holds is cleared as it is resolved.
    fn ensure_layer(ctx: &EditorContext, core: &MissionDocCore) -> String {
        let ensured = website_map_engine::data::store::operations::entity::ensure_layer(
            core,
            ctx.active_layer.get_untracked(),
            DEFAULT_LAYER_ID,
            DEFAULT_LAYER_NAME,
        );
        if ensured.active_layer_was_stale {
            ctx.active_layer.set(None);
        }
        ensured.layer_id
    }

    /// Resolve the folder a new entity is filed under without a context in hand. This is the form
    /// the engine's hosted commands take, which is why the folder id crosses the wall as an answer
    /// rather than the engine reaching for the tree's focus itself.
    pub fn ensure_active_layer(core: &MissionDocCore) -> String {
        EDITOR_CONTEXT
            .with(|c| c.borrow().as_ref().map(|ctx| ensure_layer(ctx, core)))
            .unwrap_or_else(|| DEFAULT_LAYER_ID.to_string())
    }

    /// Create a folder as a child of the focused folder (a root when none is focused), auto-named,
    /// with its inline rename armed, and focus it. Returns the new folder's id.
    pub fn create_layer() -> Option<String> {
        let active = EDITOR_CONTEXT.with(|c| {
            c.borrow()
                .as_ref()
                .and_then(|ctx| ctx.active_layer.get_untracked())
        });
        let created = engine_ops::create_layer(active)?;
        set_active_layer(Some(created.clone()));
        Some(created)
    }

    /// Delete a folder and its whole subtree, dropping the focus when it was the focused folder —
    /// a focus on a folder that no longer exists would file the next place into nothing.
    pub fn delete_layer(id: &str) -> bool {
        let did = engine_ops::delete_layer(id);
        if did {
            let focused = EDITOR_CONTEXT.with(|c| {
                c.borrow()
                    .as_ref()
                    .and_then(|ctx| ctx.active_layer.get_untracked())
            });
            if focused.as_deref() == Some(id) {
                set_active_layer(None);
            }
        }
        did
    }
}

#[cfg(target_arch = "wasm32")]
pub use active_folder::{create_layer, delete_layer, ensure_active_layer, set_active_layer};


#[cfg(test)]
#[path = "tests/outliner_model/outliner_hierarchy_visibility_and_comments.rs"]
mod outliner_hierarchy_visibility_and_comments;
