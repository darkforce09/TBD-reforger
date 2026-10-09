//! The folder-click selection rules and the group-icon rule of the Editor Layers outliner: which
//! slots a folder click selects (direct children, or the whole subtree), that a hidden layer
//! still selects what the document holds, and which folders hold slots.

use super::*;
use mission_creator_state::outliner_model::build_outliner;
use mission_document::ids::LayerId;
use mission_operations::rows::LayerRow;
use mission_operations::rows::SlotRow;

fn slot(id: &str) -> SlotRow {
    SlotRow {
        id: id.into(),
        role: "Rifleman".to_string(),
    }
}
fn layer(id: &str, parent: Option<&str>, ents: &[&str]) -> LayerRow {
    LayerRow {
        id: id.into(),
        name: format!("{id}-name"),
        parent_id: parent.map(Into::into),
        entity_ids: ents.iter().map(|s| (*s).to_string()).collect(),
        hidden: false,
        locked: false,
    }
}

/// A three-level fixture: root(a1,a2) → child(b1) → grandchild(c1).
fn nested() -> Vec<LayerRow> {
    vec![
        layer("root", None, &["a1", "a2"]),
        layer("child", Some("root"), &["b1"]),
        layer("grand", Some("child"), &["c1"]),
    ]
}

// ── SEL-LAYER-CHILDREN-001 ────────────────────────────────────────────────────────────────

#[test]
fn direct_children_are_own_entity_ids_only() {
    let layers = nested();
    // The folder's DIRECT slot children = its own `entityIds`, in order — NOT the subtree.
    assert_eq!(
        layer_direct_slot_children(&layers, &LayerId::new("root")),
        vec!["a1", "a2"]
    );
    assert_eq!(
        layer_direct_slot_children(&layers, &LayerId::new("child")),
        vec!["b1"]
    );
    assert_eq!(
        layer_direct_slot_children(&layers, &LayerId::new("grand")),
        vec!["c1"]
    );
}

#[test]
fn direct_children_unknown_layer_is_empty() {
    assert!(layer_direct_slot_children(&nested(), &LayerId::new("nope")).is_empty());
}

// ── SEL-LAYER-DESC-001 ────────────────────────────────────────────────────────────────────

#[test]
fn descendants_walk_the_whole_subtree() {
    let layers = nested();
    // root's descendants = root's own slots + child's + grandchild's (recursion is the point).
    assert_eq!(
        layer_descendant_slots(&layers, &LayerId::new("root")),
        vec!["a1", "a2", "b1", "c1"]
    );
    // child's subtree stops above root but still reaches the grandchild.
    assert_eq!(
        layer_descendant_slots(&layers, &LayerId::new("child")),
        vec!["b1", "c1"]
    );
    // a leaf folder's subtree is just itself.
    assert_eq!(
        layer_descendant_slots(&layers, &LayerId::new("grand")),
        vec!["c1"]
    );
}

/// FIRED-ONCE (perturb / fail / restore): the descendant walk MUST recurse, and this is the
/// rule the destructive delete (`remove_editor_layer` subtree) and SEL-LAYER-DESC-001 both
/// stand on. To prove the assertion has teeth, PERTURB the fixture to break the parent chain
/// (grandchild reparented off the subtree), assert the walk then MISSES `c1` (the FAIL the
/// test would catch if the logic ever stopped recursing), then RESTORE the correct chain and
/// assert `c1` is back. If `layer_descendant_slots` were a direct-children-only lookup, the
/// FIRST (perturbed) and correct cases would be identical and this test could not tell them
/// apart — so the negative arm is what makes the recursion gate real.
#[test]
fn descendants_recursion_gate_fires() {
    // Correct chain: grandchild reachable → c1 present.
    let ok = layer_descendant_slots(&nested(), &LayerId::new("root"));
    assert!(
        ok.contains(&"c1".to_string()),
        "baseline reaches grandchild"
    );

    // PERTURB: detach `grand` from the subtree (parent → an unrelated root).
    let mut perturbed = nested();
    perturbed.push(layer("other", None, &[]));
    for l in &mut perturbed {
        if l.id == "grand" {
            l.parent_id = Some("other".into());
        }
    }
    let broken = layer_descendant_slots(&perturbed, &LayerId::new("root"));
    assert!(
        !broken.contains(&"c1".to_string()),
        "PERTURBED: with the chain cut, the subtree walk must NOT reach the grandchild's slot \
         — this is the failure the recursion gate exists to prevent"
    );
    // still reaches the intact level.
    assert!(broken.contains(&"b1".to_string()), "child level intact");

    // RESTORE: the intact fixture reaches the grandchild again.
    let restored = layer_descendant_slots(&nested(), &LayerId::new("root"));
    assert!(
        restored.contains(&"c1".to_string()),
        "RESTORE: grandchild back"
    );
}

#[test]
fn descendants_cycle_guarded() {
    // A malformed parentId cycle (root ↔ child) must terminate, not hang.
    let layers = vec![
        layer("root", Some("child"), &["a1"]),
        layer("child", Some("root"), &["b1"]),
    ];
    let got = layer_descendant_slots(&layers, &LayerId::new("root"));
    assert!(got.contains(&"a1".to_string()) && got.contains(&"b1".to_string()));
}

// ── The selection reads the UNFILTERED doc (the T-715 non-regression contract) ────────────

#[test]
fn selection_reads_unfiltered_doc_hidden_layer_still_selects() {
    // A HIDDEN layer's slots are dropped by `materialize()` (and so by `slot_rows`), which is
    // the T-715 defect lane. The selection helpers read `LayerRow.entity_ids` (from
    // `small_maps_json`, unfiltered), so a hidden folder STILL selects its slots — folder-click
    // selects what the DOC contains, not what the filtered view shows.
    let mut layers = nested();
    for l in &mut layers {
        if l.id == "child" {
            l.hidden = true; // child (and its grandchild) would vanish from the render SoA
        }
    }
    // Direct + descendant selection are unaffected by the hidden flag.
    assert_eq!(
        layer_direct_slot_children(&layers, &LayerId::new("child")),
        vec!["b1"]
    );
    assert_eq!(
        layer_descendant_slots(&layers, &LayerId::new("root")),
        vec!["a1", "a2", "b1", "c1"],
        "a hidden sub-layer's slots are still part of what the parent folder contains"
    );
}

// ── SEL-GROUP-ICON-001 ────────────────────────────────────────────────────────────────────

#[test]
fn group_icon_distinguishes_slot_holders_from_grouping_folders() {
    // `parent` groups only sub-folders; `leaf` directly holds a slot.
    let layers = vec![
        layer("parent", None, &[]),
        layer("leaf", Some("parent"), &["s1"]),
    ];
    assert!(
        !folder_holds_slots(&layers, "parent"),
        "pure grouping folder"
    );
    assert!(folder_holds_slots(&layers, "leaf"), "directly holds a slot");

    // The render-side set (built from the OutlinerNode tree) agrees: only `leaf` is flagged.
    let tree = build_outliner(&layers, &[slot("s1")]);
    let holders = folders_holding_slots(&tree);
    assert!(holders.contains("leaf"));
    assert!(!holders.contains("parent"));
}

#[test]
fn group_icon_set_walks_nested_folders() {
    // Nested render set must find a slot-holder at any depth (walk, not top-level only).
    let layers = nested(); // root & child & grand all hold ≥1 slot
    let tree = build_outliner(&layers, &[slot("a1"), slot("a2"), slot("b1"), slot("c1")]);
    let holders = folders_holding_slots(&tree);
    for id in ["root", "child", "grand"] {
        assert!(holders.contains(id), "{id} directly holds a slot");
    }
}
