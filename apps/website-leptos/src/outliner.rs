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
}
