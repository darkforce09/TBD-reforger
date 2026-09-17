//! Asset catalog side vehicle and merged catalog tests.

use super::fixtures::*;
use super::*;

#[test]
fn side_filter_excludes_cross_side_characters() {
    let mut items = golden_items();
    items.push(character_row(
        "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
        "USSR Rifleman",
        "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
    ));
    items.push(character_row(
        "{84B40583F4D1B7A3}Prefabs/Characters/Factions/INDFOR/FIA/Character_FIA_Rifleman.et",
        "FIA Rifleman",
        "ArmaReforger/Characters/Factions/INDFOR/FIA/Rifleman",
    ));

    let blufor = leaf_labels(&build_catalog_tree(&items, "BLUFOR"));
    assert!(
        blufor.iter().any(|l| l == "US Rifleman"),
        "BLUFOR must keep US Army leaves"
    );
    assert!(
        !blufor
            .iter()
            .any(|l| l.contains("USSR") || l.contains("FIA")),
        "BLUFOR chip must not accept USSR/FIA — got {blufor:?}"
    );

    let opfor = leaf_labels(&build_catalog_tree(&items, "OPFOR"));
    assert_eq!(opfor, vec!["USSR Rifleman".to_string()]);
    assert!(
        !opfor.iter().any(|l| l.starts_with("US ")),
        "OPFOR chip must not accept NATO leaves — got {opfor:?}"
    );

    let indfor = leaf_labels(&build_catalog_tree(&items, "INDFOR"));
    assert_eq!(indfor, vec!["FIA Rifleman".to_string()]);

    assert!(build_catalog_tree(&items, "").is_empty());
    assert!(build_catalog_tree(&items, "CIV").is_empty());
}

#[test]
fn legacy_nato_root_matches_blufor_chip() {
    let items = golden_items();
    assert!(
        !build_catalog_tree(&items, "BLUFOR").is_empty(),
        "golden NATO characters must match BLUFOR"
    );
    assert!(
        build_catalog_tree(&items, "OPFOR").is_empty(),
        "golden has no OPFOR characters"
    );
    let ussr = character_row(
        "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
        "USSR Rifleman",
        "USSR/USSR_Army/Rifleman",
    );
    assert!(
        character_matches_eden_side(&ussr, "OPFOR"),
        "legacy USSR/ root must map to OPFOR"
    );
    assert!(!character_matches_eden_side(&ussr, "BLUFOR"));
}

#[test]
fn vehicle_tree_keeps_the_family_folder() {
    let tree = build_vehicle_catalog_tree(&vehicle_items());

    assert_eq!(tree.len(), 1, "one addon root");
    assert_eq!(tree[0].id, "ArmaReforger");
    assert!(tree[0].default_expanded, "addon root opens");

    let vehicles = &tree[0].children[0];
    assert_eq!(vehicles.id, "ArmaReforger/Vehicles");
    assert!(vehicles.default_expanded, "depth 1 opens too");

    let wheeled = vehicles
        .children
        .iter()
        .find(|n| n.label == "Wheeled")
        .expect("Wheeled folder");
    assert!(
        !wheeled.default_expanded,
        "depth 2 is where the author chooses"
    );

    let uaz = &wheeled.children[0];
    assert_eq!(
        uaz.id, "ArmaReforger/Vehicles/Wheeled/UAZ469",
        "the family segment survives as a folder"
    );
    assert_eq!(
        uaz.children
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        vec!["UAZ469", "UAZ469 PKM"],
        "both variants sit under their family"
    );
    assert_eq!(
        uaz.children[1].payload,
        Some(PlacePayload {
            asset_id: "{B}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_PKM.et".to_string(),
            role: "UAZ469 PKM".to_string(),
        }),
        "the drop carries the real ResourceName"
    );
}

#[test]
fn vehicle_tree_excludes_abstract_and_non_vehicle_rows() {
    let tree = build_vehicle_catalog_tree(&vehicle_items());

    fn leaves(nodes: &[CatalogNode], out: &mut Vec<String>) {
        for n in nodes {
            if n.payload.is_some() {
                out.push(n.label.clone());
            }
            leaves(&n.children, out);
        }
    }
    let mut found = Vec::new();
    leaves(&tree, &mut found);
    found.sort();

    assert_eq!(
        found,
        vec!["UAZ469".to_string(), "UAZ469 PKM".to_string()],
        "the abstract Mi8MT base and every character/gear row are excluded"
    );
    assert!(
        !tree.iter().any(|n| n.label == "NATO"),
        "the Factions tree must not leak into the Vehicles tab"
    );
}

#[test]
fn merged_tree_reaches_a_vehicle_leaf_under_the_nato_subtree() {
    let tree = build_faction_catalog_tree(&merged_items(), "BLUFOR");

    assert_eq!(tree.len(), 1, "one faction root for the BLUFOR chip");
    assert_eq!(tree[0].id, "NATO");
    assert!(tree[0].default_expanded, "the faction folder opens");

    let us_army = tree[0]
        .children
        .iter()
        .find(|n| n.id == "NATO/US_Army")
        .expect("US_Army folder holds both kinds");
    assert!(
        !us_army.default_expanded,
        "depth 1 stays folded until the author drills in"
    );

    let char_leaves: Vec<&str> = us_army
        .children
        .iter()
        .filter(|n| n.payload.is_some())
        .map(|n| n.label.as_str())
        .collect();
    assert_eq!(
        char_leaves,
        vec!["US Rifleman", "US Medic"],
        "characters file under the faction as display-name leaves"
    );

    let vehicles = us_army
        .children
        .iter()
        .find(|n| n.id == "NATO/US_Army/Vehicles")
        .expect("a Vehicles sub-folder is reachable inside NATO");
    let veh_leaves: Vec<&str> = vehicles
        .children
        .iter()
        .filter(|n| n.payload.is_some())
        .map(|n| n.label.as_str())
        .collect();
    assert_eq!(
        veh_leaves,
        vec!["M1025 Humvee (M2)", "M113 APC (M2)"],
        "the placeable vehicles are leaves under NATO — the abstract template is dropped"
    );

    assert_eq!(
        vehicles.children[0].payload,
        Some(PlacePayload {
            asset_id: "{86B7B7522A75FF8B}Prefabs/Vehicles/Wheeled/M998/M1025_M2.et".to_string(),
            role: "M1025 Humvee (M2)".to_string(),
        }),
        "a vehicle leaf drops its ResourceName, exactly like a character leaf"
    );
}

#[test]
fn merged_tree_is_side_filtered() {
    let blufor = build_faction_catalog_tree(&merged_items(), "BLUFOR");
    assert!(
        blufor.iter().all(|n| n.id == "NATO"),
        "BLUFOR shows the NATO faction only"
    );
    let mut blufor_leaves = leaf_labels(&blufor);
    blufor_leaves.sort();
    assert_eq!(
        blufor_leaves,
        vec![
            "M1025 Humvee (M2)".to_string(),
            "M113 APC (M2)".to_string(),
            "US Medic".to_string(),
            "US Rifleman".to_string(),
        ],
        "BLUFOR leaves are its characters and its vehicles, together"
    );

    let opfor = build_faction_catalog_tree(&merged_items(), "OPFOR");
    assert_eq!(leaf_labels(&opfor), vec!["USSR Rifleman".to_string()]);
    assert!(
        !opfor.iter().any(|n| n.id == "NATO"),
        "OPFOR must not surface the NATO subtree"
    );

    assert!(build_faction_catalog_tree(&merged_items(), "CIV").is_empty());
}

#[test]
fn picker_tree_spans_all_sides_and_still_drops_abstract() {
    let picker = build_picker_catalog_tree(&merged_items());
    let mut leaves = leaf_labels(&picker);
    leaves.sort();
    assert_eq!(
        leaves,
        vec![
            "M1025 Humvee (M2)".to_string(),
            "M113 APC (M2)".to_string(),
            "US Medic".to_string(),
            "US Rifleman".to_string(),
            "USSR Rifleman".to_string(), // the OPFOR leaf the side filter would have hidden
        ],
        "the picker spans both factions and drops the abstract *_base.et template"
    );
    assert!(picker.iter().any(|n| n.id == "NATO"), "NATO root present");
    assert!(picker.iter().any(|n| n.id == "USSR"), "USSR root present");
}

#[test]
fn picker_leaf_payload_is_the_canonical_resource_name_and_search_reaches_it() {
    let picker = build_picker_catalog_tree(&merged_items());
    let hits = filter_catalog(&picker, "rifle");
    let mut labels = leaf_labels(&hits);
    labels.sort();
    assert_eq!(
        labels,
        vec!["US Rifleman".to_string(), "USSR Rifleman".to_string()]
    );

    fn find<'a>(nodes: &'a [CatalogNode], label: &str) -> Option<&'a CatalogNode> {
        for n in nodes {
            if n.label == label && n.payload.is_some() {
                return Some(n);
            }
            if let Some(f) = find(&n.children, label) {
                return Some(f);
            }
        }
        None
    }
    let leaf = find(&picker, "US Rifleman").expect("US Rifleman leaf");
    assert_eq!(
        leaf.payload.as_ref().unwrap().asset_id,
        "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
        "the leaf writes the canonical resource_name assetId a drop would"
    );
}

#[test]
fn catalog_leaf_count_counts_leaves_not_folders() {
    let picker = build_picker_catalog_tree(&merged_items());
    assert_eq!(
        catalog_leaf_count(&picker),
        5,
        "five placeable leaves across both factions; folders are not counted"
    );
    assert_eq!(build_picker_catalog_tree(&[]).len(), 0);
    assert_eq!(catalog_leaf_count(&build_picker_catalog_tree(&[])), 0);
    let only_abstract = vec![vehicle_row(
        "{9999999999999999}Prefabs/Vehicles/Wheeled/M998/M998_base.et",
        "M998 base",
        "NATO/US_Army/Vehicles",
        true,
    )];
    assert_eq!(
        catalog_leaf_count(&build_picker_catalog_tree(&only_abstract)),
        0
    );
}

#[test]
fn search_spans_the_merged_tree() {
    let tree = build_faction_catalog_tree(&merged_items(), "BLUFOR");

    let by_class = filter_catalog(&tree, "class:M1025_M2");
    assert_eq!(
        leaf_labels(&by_class),
        vec!["M1025 Humvee (M2)".to_string()],
        "a class: query reaches a vehicle leaf in the merged tree"
    );
    let by_class_char = filter_catalog(&tree, "class:Character_US_Medic");
    assert_eq!(leaf_labels(&by_class_char), vec!["US Medic".to_string()]);

    assert_eq!(
        leaf_labels(&filter_catalog(&tree, "Humvee")),
        vec!["M1025 Humvee (M2)".to_string()]
    );
}
