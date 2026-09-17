use super::fixtures::*;
use super::*;

/// The exact tree the fixture must yield: NATO (expanded) > US_Army > the 8 character leaves in
/// `sort_order` order. Pins every ported rule at once.
#[test]
fn golden_yields_nato_us_army_and_eight_leaves() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");

    assert_eq!(tree.len(), 1, "one root faction folder");
    let nato = &tree[0];
    assert_eq!(nato.id, "NATO");
    assert_eq!(nato.label, "NATO");
    assert!(nato.default_expanded, "depth-0 folders open by default");
    assert!(nato.payload.is_none(), "folders are not placeable");

    assert_eq!(nato.children.len(), 1, "one sub-folder, no Rifleman folder");
    let army = &nato.children[0];
    assert_eq!(
        army.id, "NATO/US_Army",
        "folder id is the accumulated prefix"
    );
    assert_eq!(army.label, "US_Army");
    assert!(!army.default_expanded, "only depth 0 opens by default");

    let labels: Vec<&str> = army.children.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "US Rifleman",
            "US Grenadier",
            "US Medic",
            "US Automatic Rifleman",
            "US Machine Gunner",
            "US Platoon Leader",
            "US Light Anti-Tank",
            "US Engineer",
        ],
        "leaves are display_name, in the API's sort_order array order"
    );
}

/// Rule 4 + the payload contract: a leaf's id AND its drop `asset_id` are the full Enfusion
/// ResourceName, and its `role` is the display name.
#[test]
fn leaf_id_and_payload_carry_the_resource_name() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    let rifleman = &tree[0].children[0].children[0];
    let expected =
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";
    assert_eq!(rifleman.id, expected);
    assert_eq!(
        rifleman.payload,
        Some(PlacePayload {
            asset_id: expected.to_string(),
            role: "US Rifleman".to_string(),
        })
    );
    assert!(rifleman.children.is_empty());
}

/// Rule 1: the golden's 13 `gear_*` rows must not reach the map palette. Proven by count, so the
/// test fails if the filter is dropped (21 rows would yield extra folders/leaves).
#[test]
fn gear_rows_are_excluded() {
    let items = golden_items();
    assert_eq!(items.len(), 21, "golden row count");
    let characters = items.iter().filter(|i| i.kind == "character").count();
    assert_eq!(characters, 8);

    let tree = build_catalog_tree(&items, "BLUFOR");
    let leaves = tree[0].children[0].children.len();
    assert_eq!(leaves, 8, "only character rows are placed");
    // The gear categories (NATO/Uniform, NATO/Vest, …) would have added sibling folders.
    assert_eq!(tree[0].children.len(), 1, "no gear folders under NATO");
}

/// T-172 B9 — search filter: descendant match prunes siblings, folder self-match keeps the
/// whole subtree, empty query is identity, no match → empty.
#[test]
fn filter_catalog_rules() {
    let tree = build_catalog_tree(&golden_items(), "BLUFOR");
    assert_eq!(filter_catalog(&tree, "  "), tree, "empty query = identity");

    let rifle = filter_catalog(&tree, "rifleman");
    assert_eq!(rifle.len(), 1, "NATO kept via descendant");
    let leaves = &rifle[0].children[0].children;
    assert!(!leaves.is_empty() && leaves.len() < 8, "siblings pruned");
    assert!(leaves
        .iter()
        .all(|c| c.label.to_lowercase().contains("rifleman")));

    let nato = filter_catalog(&tree, "nato");
    assert_eq!(nato, tree, "folder self-match keeps the full subtree");

    assert!(filter_catalog(&tree, "zzz-none").is_empty());
}
