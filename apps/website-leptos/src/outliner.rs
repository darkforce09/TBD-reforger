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

/// An `editorLayers` row, as carried by the doc's `small_maps_json()` → `editorLayersById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerRow {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub entity_ids: Vec<String>,
}

/// The two slot fields the tree needs, adapted from the materialized SoA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotRow {
    pub id: String,
    pub role: String,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlinerNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub children: Vec<OutlinerNode>,
}

fn slot_node(s: &SlotRow) -> OutlinerNode {
    OutlinerNode {
        id: s.id.clone(),
        label: if s.role.is_empty() {
            SLOT_FALLBACK_LABEL.to_string()
        } else {
            s.role.clone()
        },
        kind: NodeKind::Slot,
        children: Vec::new(),
    }
}

/// Build the outliner: the "Unfiled" pseudo-root (when any slot is filed nowhere) followed by the
/// real root layers. See the module docs for the divergences and the ordering rule.
#[must_use]
pub fn build_outliner(layers: &[LayerRow], slots: &[SlotRow]) -> Vec<OutlinerNode> {
    let mut out: Vec<OutlinerNode> = Vec::new();

    // Reverse index, matching `MissionDocCore::materialize` (`store.rs:206-221`): a slot belongs to
    // the FIRST layer whose `entityIds` lists it; one in none is unfiled.
    let filed: HashSet<&str> = layers
        .iter()
        .flat_map(|l| l.entity_ids.iter().map(String::as_str))
        .collect();

    let mut unfiled: Vec<&SlotRow> = slots
        .iter()
        .filter(|s| !filed.contains(s.id.as_str()))
        .collect();
    unfiled.sort_by(|a, b| a.id.cmp(&b.id)); // deterministic; materialize order is arbitrary
    if !unfiled.is_empty() {
        out.push(OutlinerNode {
            id: UNFILED_ID.to_string(),
            label: format!("Unfiled ({})", unfiled.len()),
            kind: NodeKind::Unfiled,
            children: unfiled.into_iter().map(slot_node).collect(),
        });
    }

    for root in layers.iter().filter(|l| l.parent_id.is_none()) {
        // `seen` guards a malformed `parentId` cycle. The core's `reparent_editor_layer` is
        // cycle-guarded (`store.rs:826`), so this is belt-and-braces — but an unguarded recursion
        // would hang the tab rather than render wrong, which is not a trade worth taking.
        let mut seen = HashSet::new();
        out.push(build_layer(root, layers, slots, &mut seen));
    }

    out
}

fn build_layer<'a>(
    layer: &'a LayerRow,
    layers: &'a [LayerRow],
    slots: &[SlotRow],
    seen: &mut HashSet<&'a str>,
) -> OutlinerNode {
    let mut children: Vec<OutlinerNode> = Vec::new();
    if seen.insert(layer.id.as_str()) {
        // Child folders first, then this folder's slots — React's `[...childFolders, ...entityNodes]`.
        for child in layers
            .iter()
            .filter(|l| l.parent_id.as_deref() == Some(layer.id.as_str()))
        {
            children.push(build_layer(child, layers, slots, seen));
        }
        // `entityIds` order (React parity). A dangling id (slot deleted, layer not yet patched) is
        // skipped, mirroring React's `.filter((s): s is Slot => Boolean(s))`.
        for eid in &layer.entity_ids {
            if let Some(s) = slots.iter().find(|s| &s.id == eid) {
                children.push(slot_node(s));
            }
        }
    }

    OutlinerNode {
        id: layer.id.clone(),
        label: layer.name.clone(),
        kind: NodeKind::Folder,
        children,
    }
}

/* ───────────────────────────── T-168 — ORBAT tree ───────────────────────────── */

/// A `factions` row from the doc's `small_maps_json()` → `factionsById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactionRow {
    pub id: String,
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
                    .map(slot_node)
                    .collect();
                OutlinerNode {
                    id: sq.id.clone(),
                    label: format!("{} ({})", sq.name, slot_children.len()),
                    kind: NodeKind::Squad,
                    children: slot_children,
                }
            })
            .collect();
        out.push(OutlinerNode {
            id: f.id.clone(),
            label: f.name.clone(),
            kind: NodeKind::Faction,
            children: squad_nodes,
        });
    }
    out
}

/* ───────────────────────────── T-169 — flattened rows for windowing ───────────────────────────── */

/// One flattened tree row (pre-order): the node's identity + its nesting depth. The windowed
/// renderer slices a `Vec<FlatRow>` and draws only the visible span (React `flattenOutliner`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatRow {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub depth: usize,
}

/// Flatten a tree to pre-order rows (parent before its children). Every node becomes exactly one
/// row — the window operates on this flat list, not the nested `OutlinerNode`s.
#[must_use]
pub fn flatten(nodes: &[OutlinerNode]) -> Vec<FlatRow> {
    let mut out = Vec::new();
    fn walk(nodes: &[OutlinerNode], depth: usize, out: &mut Vec<FlatRow>) {
        for n in nodes {
            out.push(FlatRow {
                id: n.id.clone(),
                label: n.label.clone(),
                kind: n.kind,
                depth,
            });
            walk(&n.children, depth + 1, out);
        }
    }
    walk(nodes, 0, &mut out);
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
        LayerRow {
            id: id.to_string(),
            name: name.to_string(),
            parent_id: parent.map(str::to_string),
            entity_ids: ents.iter().map(|s| (*s).to_string()).collect(),
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
            name: name.into(),
            squad_ids: squads.iter().map(|s| (*s).to_string()).collect(),
        }
    }
    fn squad(id: &str, name: &str, faction: &str, slots: &[&str]) -> SquadRow {
        SquadRow {
            id: id.into(),
            name: name.into(),
            faction_id: faction.into(),
            slot_ids: slots.iter().map(|s| (*s).to_string()).collect(),
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
        let squads = vec![squad("sq1", "Alpha", "f1", &["s1", "s0"])];
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

    /// Dangling squad/slot ids are skipped, not rendered blank.
    #[test]
    fn orbat_skips_dangling_ids() {
        let factions = vec![faction("f1", "US Army", &["ghostSquad", "sq1"])];
        let squads = vec![squad("sq1", "Alpha", "f1", &["ghostSlot", "s0"])];
        let tree = build_orbat(&factions, &squads, &[slot("s0", "Rifleman")]);
        assert_eq!(tree[0].children.len(), 1, "ghost squad skipped");
        assert_eq!(tree[0].children[0].children.len(), 1, "ghost slot skipped");
        assert_eq!(tree[0].children[0].children[0].id, "s0");
    }
}
