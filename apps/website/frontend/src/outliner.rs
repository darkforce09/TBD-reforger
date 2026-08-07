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

/// The virtual root's id. Not a doc id — see the module docs.
pub const UNFILED_ID: &str = "__unfiled";
/// T-169 — above this many flattened rows a tree renders windowed (React `VIRTUAL_SLOT_THRESHOLD`,
/// proven @ ~367k). Below it, the eager recursive render is cheaper and keeps native scroll simple.
pub const VIRTUAL_SLOT_THRESHOLD: usize = 50;
/// React's `label: s.role || 'Unit'` fallback (`EditorLayersSection.tsx:66`).
const SLOT_FALLBACK_LABEL: &str = "Unit";
/// T-651 — an untitled comment's row label (the `SLOT_FALLBACK_LABEL` idiom: a row must always be
/// clickable, and a blank title would render a zero-width row you cannot select to fix).
pub const COMMENT_FALLBACK_LABEL: &str = "Comment";

/// An `editorLayers` row, as carried by the doc's `small_maps_json()` → `editorLayersById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerRow {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub entity_ids: Vec<String>,
    /// T-665 — the layer's own `hidden` VIEW flag (per-layer visibility). Absent in the doc ⇒
    /// `false`. This is the layer's OWN bit, not the resolved one: [`build_outliner`] passes the
    /// inherited-hidden state to the glyph via [`OutlinerNode::hidden`] so a child under a hidden
    /// parent renders dimmed too, but the eye toggle flips only this layer's own flag.
    pub hidden: bool,
    /// T-665 — the layer's own `locked` transform-lock flag (absent ⇒ `false`); same own-vs-resolved
    /// split as [`Self::hidden`].
    pub locked: bool,
}

/// The two slot fields the tree needs, adapted from the materialized SoA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotRow {
    pub id: String,
    pub role: String,
}

/// T-651 — one `commentsById` row as the tree needs it (`PLACE-COMMENT-001`).
///
/// A comment is an **editor-only virtual entity**: it appears here, files into a layer and drags
/// like a slot, and it NEVER reaches the compiled mission (the exclusion is structural, in
/// `map-engine-core`'s `doc/store.rs` — `mission::flatten::EditorPayload` declares no `comments`
/// key). This row carries no position: the tree does not draw the map, and leaving `x`/`z` out
/// means a drag that moves a comment cannot desync a stale copy held by the outliner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentRow {
    pub id: String,
    /// ATTR-FIELD-CMT-TITLE — the row label. Empty falls back to [`COMMENT_FALLBACK_LABEL`].
    pub title: String,
    /// ATTR-FIELD-CMT-TOOLTIP — the long body, rendered as the row's hover text.
    pub tooltip: String,
}

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

/// A `factions` row from the doc's `small_maps_json()` → `factionsById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactionRow {
    pub id: String,
    /// Side / faction key (`BLUFOR` / `OPFOR` / `INDFOR`) — T-180.1; must not be dropped.
    pub key: String,
    pub name: String,
    /// Ordered squad ids under this faction (`faction.squadIds`).
    pub squad_ids: Vec<String>,
}

/// A `squads` row from the doc's `small_maps_json()` → `squadsById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquadRow {
    pub id: String,
    pub name: String,
    pub faction_id: String,
    /// Ordered slot ids in this squad (`squad.slotIds`).
    pub slot_ids: Vec<String>,
    /// T-180.6 — `squad.leaderSlotId` (empty when absent); drives ORBAT SL badge.
    pub leader_slot_id: String,
    /// T-180.7 — `squad.vehicleIds` (badge when non-empty; attach wiring in T-180.8).
    pub vehicle_ids: Vec<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(id: &str, role: &str) -> SlotRow {
        SlotRow {
            id: id.to_string(),
            role: role.to_string(),
        }
    }
    fn layer(id: &str, name: &str, parent: Option<&str>, ents: &[&str]) -> LayerRow {
        layer_flags(id, name, parent, ents, false, false)
    }

    /// T-665 — a layer row with explicit `hidden`/`locked` flags.
    fn layer_flags(
        id: &str,
        name: &str,
        parent: Option<&str>,
        ents: &[&str],
        hidden: bool,
        locked: bool,
    ) -> LayerRow {
        LayerRow {
            id: id.to_string(),
            name: name.to_string(),
            parent_id: parent.map(str::to_string),
            entity_ids: ents.iter().map(|s| (*s).to_string()).collect(),
            hidden,
            locked,
        }
    }

    /// The boot state: 8 seed slots, zero layers (`seed_random` files nothing). Every slot must be
    /// reachable under Unfiled, id-sorted — this is what makes the dock non-empty at boot and the
    /// gate's "click row 0 → s0" assertion exact.
    #[test]
    fn seed_boot_state_lists_all_slots_under_unfiled_id_sorted() {
        // Deliberately out of order: `materialize()` row order is arbitrary.
        let slots: Vec<SlotRow> = ["s3", "s0", "s7", "s1", "s5", "s2", "s6", "s4"]
            .iter()
            .map(|id| slot(id, "Rifleman"))
            .collect();

        let tree = build_outliner(&[], &slots);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, UNFILED_ID);
        assert_eq!(tree[0].kind, NodeKind::Unfiled);
        assert_eq!(tree[0].label, "Unfiled (8)");
        let ids: Vec<&str> = tree[0].children.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7"]);
        assert!(tree[0].children.iter().all(|n| n.label == "Rifleman"));
    }

    /// After the first place: the new slot is filed under the lazily-minted default layer and leaves
    /// Unfiled, which keeps the remaining seeds.
    #[test]
    fn filed_slot_leaves_unfiled_and_appears_in_its_layer() {
        let slots = vec![slot("s0", "Rifleman"), slot("n0", "US Rifleman")];
        let layers = vec![layer("layer-1", "Layer 1", None, &["n0"])];

        let tree = build_outliner(&layers, &slots);

        assert_eq!(tree.len(), 2, "Unfiled then the real root layer");
        assert_eq!(tree[0].id, UNFILED_ID);
        assert_eq!(tree[0].label, "Unfiled (1)");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].id, "s0");

        assert_eq!(tree[1].id, "layer-1");
        assert_eq!(tree[1].kind, NodeKind::Folder);
        assert_eq!(tree[1].children.len(), 1);
        assert_eq!(tree[1].children[0].id, "n0");
        assert_eq!(tree[1].children[0].label, "US Rifleman");
    }

    /// No Unfiled root at all once every slot is filed — the React-parity shape.
    #[test]
    fn no_unfiled_root_when_everything_is_filed() {
        let slots = vec![slot("n0", "US Rifleman")];
        let layers = vec![layer("layer-1", "Layer 1", None, &["n0"])];
        let tree = build_outliner(&layers, &slots);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, "layer-1");
    }

    /// React's `[...childFolders, ...entityNodes]` order + `parentId` nesting.
    #[test]
    fn child_folders_precede_slots_and_nest_by_parent_id() {
        let slots = vec![slot("a", "Alpha"), slot("b", "Bravo")];
        let layers = vec![
            layer("root", "Root", None, &["a"]),
            layer("kid", "Kid", Some("root"), &["b"]),
        ];
        let tree = build_outliner(&layers, &slots);

        assert_eq!(tree.len(), 1, "only the root layer is top-level");
        let root = &tree[0];
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].id, "kid", "child folder first");
        assert_eq!(root.children[0].children[0].id, "b");
        assert_eq!(root.children[1].id, "a", "then this folder's slots");
    }

    /// Empty role → React's `'Unit'` fallback.
    #[test]
    fn empty_role_falls_back_to_unit() {
        let tree = build_outliner(&[], &[slot("s0", "")]);
        assert_eq!(tree[0].children[0].label, "Unit");
    }

    /// A slot id listed by a layer but absent from the doc is skipped, not rendered blank.
    #[test]
    fn dangling_entity_id_is_skipped() {
        let layers = vec![layer("layer-1", "Layer 1", None, &["ghost", "s0"])];
        let tree = build_outliner(&layers, &[slot("s0", "Rifleman")]);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].id, "s0");
    }

    /// A `parentId` cycle must terminate rather than hang the tab.
    #[test]
    fn parent_id_cycle_terminates() {
        let layers = vec![
            layer("a", "A", None, &[]),
            layer("b", "B", Some("a"), &[]),
            layer("c", "C", Some("b"), &[]),
        ];
        // Force a cycle: c is b's child, and b is also c's child.
        let mut cyclic = layers.clone();
        cyclic.push(layer("b2", "B2", Some("c"), &[]));
        let tree = build_outliner(&cyclic, &[]);
        assert_eq!(tree.len(), 1, "only `a` is rooted");
    }

    fn faction(id: &str, name: &str, squads: &[&str]) -> FactionRow {
        FactionRow {
            id: id.into(),
            key: name.into(), // tests use name as display; key mirrors for row shape
            name: name.into(),
            squad_ids: squads.iter().map(|s| (*s).to_string()).collect(),
        }
    }
    fn squad(id: &str, name: &str, faction: &str, slots: &[&str], leader: &str) -> SquadRow {
        SquadRow {
            id: id.into(),
            name: name.into(),
            faction_id: faction.into(),
            slot_ids: slots.iter().map(|s| (*s).to_string()).collect(),
            leader_slot_id: leader.into(),
            vehicle_ids: Vec::new(),
        }
    }

    /// No factions/squads (seed boot) → empty ORBAT tree.
    #[test]
    fn orbat_empty_before_any_squad() {
        assert!(build_orbat(&[], &[], &[slot("s0", "Rifleman")]).is_empty());
    }

    /// faction → squad → slot in doc (`squadIds`/`slotIds`) order; squad label carries its count.
    #[test]
    fn orbat_nests_faction_squad_slot_in_order() {
        let factions = vec![faction("f1", "US Army", &["sq1"])];
        let squads = vec![squad("sq1", "Alpha", "f1", &["s1", "s0"], "s1")];
        let slots = vec![slot("s0", "Rifleman"), slot("s1", "Squad Leader")];
        let tree = build_orbat(&factions, &squads, &slots);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].kind, NodeKind::Faction);
        assert_eq!(tree[0].label, "US Army");
        let sq = &tree[0].children[0];
        assert_eq!(sq.kind, NodeKind::Squad);
        assert_eq!(sq.label, "Alpha (2)");
        // slotIds order preserved (s1 before s0).
        let ids: Vec<&str> = sq.children.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["s1", "s0"]);
        assert!(sq.children.iter().all(|n| n.kind == NodeKind::Slot));
    }

    /// F3 — place-shaped rows (one side faction, two minted squads) both appear in the ORBAT tree.
    #[test]
    fn orbat_includes_two_squads_after_place_shaped_rows() {
        let factions = vec![faction(
            "faction-BLUFOR",
            "BLUFOR",
            &["squad-BLUFOR-1", "squad-BLUFOR-2"],
        )];
        let squads = vec![
            squad("squad-BLUFOR-1", "Squad 1", "faction-BLUFOR", &["a"], "a"),
            squad("squad-BLUFOR-2", "Squad 2", "faction-BLUFOR", &["b"], "b"),
        ];
        let slots = vec![slot("a", "Rifleman"), slot("b", "Rifleman")];
        let tree = build_orbat(&factions, &squads, &slots);
        assert_eq!(tree.len(), 1);
        let sq_ids: Vec<&str> = tree[0].children.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(sq_ids, ["squad-BLUFOR-1", "squad-BLUFOR-2"]);
    }

    /// F-L6 — SL badge flag from `leaderSlotId` only (not role / tag text).
    #[test]
    fn orbat_sl_badge_from_leader_slot_id() {
        let factions = vec![faction("f1", "BLUFOR", &["sq1"])];
        let squads = vec![squad("sq1", "Alpha", "f1", &["s0", "s1"], "s0")];
        // Role text looks like a tag — must not drive is_leader.
        let slots = vec![slot("s0", "Rifleman"), slot("s1", "SL")];
        let tree = build_orbat(&factions, &squads, &slots);
        let kids = &tree[0].children[0].children;
        assert!(kids.iter().find(|n| n.id == "s0").unwrap().is_leader);
        assert!(!kids.iter().find(|n| n.id == "s1").unwrap().is_leader);
        let flat = flatten(&tree);
        assert!(flat.iter().find(|r| r.id == "s0").unwrap().is_leader);
        assert!(!flat.iter().find(|r| r.id == "s1").unwrap().is_leader);
    }

    /// Flatten is pre-order (parent before children) with correct depths, one row per node.
    #[test]
    fn flatten_is_preorder_with_depths() {
        // Unfiled (depth 0) → its 2 slots (depth 1); a root layer (0) → child folder (1) → slot (2).
        let slots = vec![slot("s0", "A"), slot("s1", "B"), slot("n0", "N")];
        let layers = vec![
            layer("root", "Root", None, &[]),
            layer("kid", "Kid", Some("root"), &["n0"]),
        ];
        let tree = build_outliner(&layers, &slots);
        let flat = flatten(&tree);
        // Unfiled, s0, s1, root, kid, n0 = 6 rows.
        assert_eq!(flat.len(), 6);
        assert_eq!(flat[0].kind, NodeKind::Unfiled);
        assert_eq!(flat[0].depth, 0);
        assert_eq!((flat[1].id.as_str(), flat[1].depth), ("s0", 1));
        assert_eq!((flat[3].id.as_str(), flat[3].depth), ("root", 0));
        assert_eq!((flat[4].id.as_str(), flat[4].depth), ("kid", 1));
        assert_eq!((flat[5].id.as_str(), flat[5].depth), ("n0", 2));
    }

    /// T-177 A1 — the YouTube-guide continuation vector. `ancestors[k]` = "column k continues below
    /// this row"; roots are `[]`, a non-last parent leads a child's vector with `true`, and last
    /// children trim to `false`. Self-contained per row so the windowed slice needs no sibling peek.
    #[test]
    fn flatten_visible_computes_ancestor_continuation() {
        fn node(id: &str, kind: NodeKind, children: Vec<OutlinerNode>) -> OutlinerNode {
            OutlinerNode {
                id: id.to_string(),
                label: id.to_string(),
                kind,
                children,
                is_leader: false,
                hidden: false,
                locked: false,
                hidden_effective: false,
                locked_effective: false,
                tooltip: String::new(),
            }
        }
        // Root(+sib Root2) → [ChildA(+sib ChildB) → GrandA, ChildB(last) → Leaf]; Root2(last) → Leaf2.
        let tree = vec![
            node(
                "Root",
                NodeKind::Folder,
                vec![
                    node(
                        "ChildA",
                        NodeKind::Folder,
                        vec![node("GrandA", NodeKind::Slot, vec![])],
                    ),
                    node(
                        "ChildB",
                        NodeKind::Folder,
                        vec![node("Leaf", NodeKind::Slot, vec![])],
                    ),
                ],
            ),
            node(
                "Root2",
                NodeKind::Folder,
                vec![node("Leaf2", NodeKind::Slot, vec![])],
            ),
        ];
        let flat = flatten(&tree);
        let by = |id: &str| flat.iter().find(|r| r.id == id).unwrap().ancestors.clone();
        let ids = |id: &str| flat.iter().find(|r| r.id == id).unwrap().guide_ids.clone();
        assert_eq!(by("Root"), Vec::<bool>::new(), "roots draw no guide column");
        assert_eq!(ids("Root"), Vec::<String>::new());
        assert_eq!(
            by("ChildA"),
            vec![true],
            "non-last child's own connector continues"
        );
        assert_eq!(ids("ChildA"), vec!["Root".to_string()]);
        assert_eq!(
            by("GrandA"),
            vec![true, false],
            "non-last parent spine (true) + last child (false)"
        );
        assert_eq!(
            ids("GrandA"),
            vec!["Root".to_string(), "ChildA".to_string()]
        );
        assert_eq!(by("ChildB"), vec![false], "last child trims its connector");
        assert_eq!(
            by("Leaf"),
            vec![false, false],
            "last-child parent spine blank + last child"
        );
        assert_eq!(by("Root2"), Vec::<bool>::new());
        assert_eq!(by("Leaf2"), vec![false], "only child trims");
        assert_eq!(ids("Leaf2"), vec!["Root2".to_string()]);
    }

    /// Dangling squad/slot ids are skipped, not rendered blank.
    #[test]
    fn orbat_skips_dangling_ids() {
        let factions = vec![faction("f1", "US Army", &["ghostSquad", "sq1"])];
        let squads = vec![squad("sq1", "Alpha", "f1", &["ghostSlot", "s0"], "s0")];
        let tree = build_orbat(&factions, &squads, &[slot("s0", "Rifleman")]);
        assert_eq!(tree[0].children.len(), 1, "ghost squad skipped");
        assert_eq!(tree[0].children[0].children.len(), 1, "ghost slot skipped");
        assert_eq!(tree[0].children[0].children[0].id, "s0");
    }

    /// G1 — dialog class is near-fullscreen (`w-[min(` / `max-w-6xl`), not `max-w-xl`-only.
    #[test]
    fn orbat_manager_dialog_class_near_fullscreen() {
        assert!(
            ORBAT_MANAGER_DIALOG_CLASS.contains("w-[min(")
                || ORBAT_MANAGER_DIALOG_CLASS.contains("max-w-6xl")
                || ORBAT_MANAGER_DIALOG_CLASS.contains("max-w-4xl"),
            "{ORBAT_MANAGER_DIALOG_CLASS}"
        );
        assert!(
            !ORBAT_MANAGER_DIALOG_CLASS.contains("max-w-xl"),
            "max-w-xl must not be the width constraint"
        );
    }

    /// G7 — empty factions ⇒ empty filtered tree (no Stitch sample SoT).
    #[test]
    fn orbat_manager_empty_doc_empty_tree() {
        let filtered = filter_orbat_squads_by_side_key(&[], &[], &[], "BLUFOR");
        assert!(filtered.is_empty());
        assert!(!ORBAT_MANAGER_EMPTY.contains("L85A3"));
        assert!(!ORBAT_MANAGER_EMPTY.contains("US 1980s"));
    }

    /// G8 — OPFOR tab filters by FactionRow.key, not name substring.
    #[test]
    fn orbat_side_tab_filters_by_faction_key() {
        let factions = vec![
            FactionRow {
                id: "faction-OPFOR".into(),
                key: "OPFOR".into(),
                name: "Enemy Force BLUFOR-looking".into(), // name must not match BLUFOR tab
                squad_ids: vec!["sq-op".into()],
            },
            FactionRow {
                id: "faction-BLUFOR".into(),
                key: "BLUFOR".into(),
                name: "OPFOR string in name".into(), // name must not match OPFOR tab
                squad_ids: vec!["sq-blu".into()],
            },
        ];
        let squads = vec![
            squad("sq-op", "Op Squad", "faction-OPFOR", &["a"], "a"),
            squad("sq-blu", "Blu Squad", "faction-BLUFOR", &["b"], "b"),
        ];
        let slots = vec![slot("a", "Rifleman"), slot("b", "Rifleman")];
        let opfor = filter_orbat_squads_by_side_key(&factions, &squads, &slots, "OPFOR");
        assert_eq!(opfor.len(), 1);
        assert_eq!(opfor[0].id, "sq-op");
        let blufor = filter_orbat_squads_by_side_key(&factions, &squads, &slots, "BLUFOR");
        assert_eq!(blufor.len(), 1);
        assert_eq!(blufor[0].id, "sq-blu");
    }

    /// T-172 B6 — collapse hides the subtree, keeps the container row, and `has_children` is
    /// true only for containers with kids; depths of surviving rows are unchanged.
    #[test]
    fn flatten_visible_collapse_hides_subtree() {
        let layers = vec![
            layer("l1", "Alpha", None, &["s0"]),
            layer("l2", "Bravo", Some("l1"), &["s1"]),
        ];
        let slots = vec![slot("s0", "SL"), slot("s1", "AR")];
        let tree = build_outliner(&layers, &slots);
        let all = flatten(&tree);
        // l1 > [s0, l2 > s1] — full walk has 4 rows (order: folders/slots per build rules).
        assert_eq!(all.len(), 4);
        let l1_row = all.iter().find(|r| r.id == "l1").unwrap();
        assert!(l1_row.has_children);
        assert!(!all.iter().find(|r| r.id == "s1").unwrap().has_children);

        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert("l1".to_string());
        let vis = flatten_visible(&tree, &collapsed);
        assert_eq!(vis.len(), 1, "collapsed root leaves only its own row");
        assert_eq!(vis[0].id, "l1");
        assert_eq!(vis[0].depth, 0);

        // Collapsing the nested folder keeps l1's direct children visible.
        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert("l2".to_string());
        let vis = flatten_visible(&tree, &collapsed);
        assert!(vis.iter().any(|r| r.id == "l2"));
        assert!(!vis.iter().any(|r| r.id == "s1"));
    }

    /* ───────────────────────────── T-665 — layer flags in the tree ───────────────────────────── */

    /// A layer's own `hidden` flag reaches its Folder row AND dims its own slots (own == effective),
    /// while a sibling layer with no flag stays fully visible. Fired once: flag present vs absent.
    #[test]
    fn own_hidden_flag_marks_folder_and_its_slots() {
        let layers = vec![
            layer_flags("h", "Hidden", None, &["s0"], true, false),
            layer("v", "Visible", None, &["s1"]),
        ];
        let slots = vec![slot("s0", "SL"), slot("s1", "AR")];
        let rows = flatten(&build_outliner(&layers, &slots));

        let hf = rows.iter().find(|r| r.id == "h").unwrap();
        assert!(
            hf.hidden && hf.hidden_effective,
            "own+effective on the folder"
        );
        let s0 = rows.iter().find(|r| r.id == "s0").unwrap();
        assert!(s0.hidden_effective, "slot under the hidden layer is dimmed");
        assert!(!s0.hidden, "a slot has no OWN flag");

        let vf = rows.iter().find(|r| r.id == "v").unwrap();
        assert!(!vf.hidden && !vf.hidden_effective, "sibling stays visible");
        let s1 = rows.iter().find(|r| r.id == "s1").unwrap();
        assert!(!s1.hidden_effective, "sibling's slot stays visible");
    }

    /// INHERITANCE, resolved at build time: a hidden/locked PARENT dims + lock-marks a child folder
    /// and the child's slots, while the child rows carry NO own flag (never copied down). Un-flagging
    /// is just the absence — this asserts the propagation the same way the store resolves it at read.
    #[test]
    fn child_folder_and_slots_inherit_parent_hidden_and_locked() {
        let layers = vec![
            layer_flags("p", "Parent", None, &[], true, true),
            layer("c", "Child", Some("p"), &["s0"]),
        ];
        let rows = flatten(&build_outliner(&layers, &vec![slot("s0", "Rifleman")]));

        let cf = rows.iter().find(|r| r.id == "c").unwrap();
        assert!(!cf.hidden && !cf.locked, "child folder has no OWN flags");
        assert!(
            cf.hidden_effective && cf.locked_effective,
            "child folder inherits parent hidden+locked"
        );
        let s0 = rows.iter().find(|r| r.id == "s0").unwrap();
        assert!(
            s0.hidden_effective && s0.locked_effective,
            "slot two levels down inherits both"
        );
    }

    /* ───────────────── T-651 — editor comments / annotations (PLACE-COMMENT-001) ───────────────── */

    fn comment(id: &str, title: &str, tooltip: &str) -> CommentRow {
        CommentRow {
            id: id.to_string(),
            title: title.to_string(),
            tooltip: tooltip.to_string(),
        }
    }

    /// A comment files into a folder through the SAME `entityIds` array a slot does, and it sits at
    /// its authored position in that sequence rather than in a segregated block — so an operator who
    /// dropped a note between two units sees it between them.
    #[test]
    fn a_filed_comment_sits_in_entity_ids_order_beside_slots() {
        let layers = vec![layer("L", "Layer", None, &["s1", "cmt-1", "s2"])];
        let slots = vec![slot("s1", "SL"), slot("s2", "Rifleman")];
        let comments = vec![comment("cmt-1", "Assembly area", "form up here")];
        let tree = build_outliner_with_comments(&layers, &slots, &comments);

        assert_eq!(tree.len(), 1, "no Unfiled root — everything is filed");
        let kids = &tree[0].children;
        assert_eq!(
            kids.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            vec!["s1", "cmt-1", "s2"],
            "the comment keeps its authored slot in the sequence"
        );
        assert_eq!(kids[1].kind, NodeKind::Comment);
        assert_eq!(kids[1].label, "Assembly area", "the title is the row label");
        assert_eq!(kids[1].tooltip, "form up here");
        assert!(kids[1].children.is_empty(), "a comment is a leaf");
    }

    /// A comment listed in no folder lands in the Unfiled pseudo-root, whose count covers BOTH kinds
    /// (a header reading "Unfiled (1)" over two rows is the bug this pins).
    #[test]
    fn unfiled_comments_join_the_pseudo_root_and_are_counted() {
        let tree = build_outliner_with_comments(
            &[],
            &[slot("s1", "SL")],
            &[comment("cmt-2", "B", ""), comment("cmt-1", "A", "")],
        );
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].kind, NodeKind::Unfiled);
        assert_eq!(
            tree[0].label, "Unfiled (3)",
            "one slot + two comments — the header counts BOTH kinds"
        );
        assert_eq!(
            tree[0]
                .children
                .iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>(),
            vec!["s1", "cmt-1", "cmt-2"],
            "slots first, then comments sorted by id (Unfiled has no authored order)"
        );
    }

    /// An Unfiled root appears for comments ALONE — a mission whose only annotation is unfiled must
    /// still show it, not silently swallow it because there are no unfiled slots.
    #[test]
    fn a_lone_unfiled_comment_still_gets_the_pseudo_root() {
        let layers = vec![layer("L", "Layer", None, &["s1"])];
        let tree = build_outliner_with_comments(
            &layers,
            &[slot("s1", "SL")],
            &[comment("cmt-1", "note", "")],
        );
        assert_eq!(tree[0].kind, NodeKind::Unfiled);
        assert_eq!(tree[0].label, "Unfiled (1)");
        assert_eq!(tree[0].children[0].kind, NodeKind::Comment);
    }

    /// An untitled comment still renders a clickable row (the `SLOT_FALLBACK_LABEL` rule) — a blank
    /// title must not produce a zero-width row you cannot select in order to fix it.
    #[test]
    fn an_untitled_comment_falls_back_to_a_label() {
        let tree = build_outliner_with_comments(&[], &[], &[comment("cmt-1", "", "body")]);
        assert_eq!(tree[0].children[0].label, COMMENT_FALLBACK_LABEL);
    }

    /// A comment does NOT inherit its folder's hidden/locked adornments: it is not in the render SoA
    /// (so "hidden" has nothing to hide) and its position is not transform-locked (see
    /// `MissionDocCore::set_comment_position`). Dimming it would advertise a refusal that does not
    /// exist — while the sibling SLOT in the same folder does inherit both, which is the contrast
    /// that makes this a decision rather than an omission.
    #[test]
    fn a_comment_does_not_inherit_hidden_or_locked_but_its_sibling_slot_does() {
        let layers = vec![layer_flags(
            "L",
            "Layer",
            None,
            &["s1", "cmt-1"],
            true,
            true,
        )];
        let tree = build_outliner_with_comments(
            &layers,
            &[slot("s1", "SL")],
            &[comment("cmt-1", "note", "")],
        );
        let kids = &tree[0].children;
        assert!(
            kids[0].hidden_effective && kids[0].locked_effective,
            "slot inherits"
        );
        assert!(
            !kids[1].hidden_effective && !kids[1].locked_effective,
            "comment does not: {:?}",
            kids[1]
        );
    }

    /// `build_outliner` (the comment-free entry point) is exactly `build_outliner_with_comments`
    /// with an empty slice — so no caller that predates comments can drift from the one that has
    /// them.
    #[test]
    fn build_outliner_is_the_empty_comment_case() {
        let layers = vec![layer("L", "Layer", None, &["s1"])];
        let slots = vec![slot("s1", "SL")];
        assert_eq!(
            build_outliner(&layers, &slots),
            build_outliner_with_comments(&layers, &slots, &[])
        );
    }

    /// The tooltip survives the flatten into windowed rows — the windowed renderer draws from
    /// `FlatRow`, so a body that stopped at `OutlinerNode` would vanish on any tree past the
    /// virtualization threshold and nowhere else (the nastiest possible way to lose it).
    #[test]
    fn flatten_carries_the_comment_tooltip_into_the_windowed_row() {
        let layers = vec![layer("L", "Layer", None, &["cmt-1"])];
        let tree =
            build_outliner_with_comments(&layers, &[], &[comment("cmt-1", "T", "long body")]);
        let rows = flatten(&tree);
        let row = rows
            .iter()
            .find(|r| r.kind == NodeKind::Comment)
            .expect("a comment row");
        assert_eq!(row.tooltip, "long body");
        assert_eq!(row.label, "T");
        assert_eq!(row.depth, 1);
        // Every other row carries an empty tooltip — the field is comment-only.
        assert!(rows
            .iter()
            .filter(|r| r.kind != NodeKind::Comment)
            .all(|r| r.tooltip.is_empty()));
    }

    /// A dangling id in `entityIds` (the comment was deleted, the folder not yet patched) is skipped
    /// exactly as a dangling slot id is — a stale reference must never panic or render a ghost row.
    #[test]
    fn a_dangling_comment_id_is_skipped_not_rendered() {
        let layers = vec![layer("L", "Layer", None, &["cmt-gone", "s1"])];
        let tree = build_outliner_with_comments(&layers, &[slot("s1", "SL")], &[]);
        assert_eq!(
            tree[0]
                .children
                .iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>(),
            vec!["s1"]
        );
    }
}
