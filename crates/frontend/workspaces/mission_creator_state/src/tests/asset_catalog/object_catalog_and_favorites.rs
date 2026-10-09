//! The object catalog tree, object alias derivation, and favourite resolution against the palette
//! builders.

use super::fixtures::*;
use super::*;

#[test]
fn object_tree_keeps_crates_and_excludes_abstract_and_characters() {
    let tree = build_object_catalog_tree(&object_items());
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
        vec![
            "AmmoBox 50cal 100rnd".to_string(),
            "AmmoBoxArsenal Equipment US Apparel".to_string(),
        ],
        "abstract crates, unregistered aliases, and character/gear rows must not reach Objects leaves"
    );
    assert!(
        !tree.iter().any(|n| n.label == "NATO"),
        "Factions tree must not leak into Objects"
    );
}

#[test]
fn derive_object_alias_slugs_display_name_and_hits_known_comp() {
    assert_eq!(
        derive_object_alias("{FA}Prefabs/Props/X.et", "Ammo Crate 5.56"),
        "prop:ammo_crate_5_56"
    );
    assert_eq!(
        derive_object_alias(
            "{E1D01D77D7F47EF3}PrefabsEditable/Auto/Compositions/Misc/SubCompositions/E_Sandbag_Barricade_US_04.et",
            "Sandbag Barricade"
        ),
        "comp:checkpoint_small"
    );
    assert_eq!(
        derive_object_alias(
            "{X}PrefabsEditable/Auto/Compositions/Misc/Foo.et",
            "Checkpoint Small"
        ),
        "comp:checkpoint_small"
    );
}

#[test]
fn favourite_resolution_mirrors_the_palette_builders() {
    let items = object_items();

    let known = "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd.et";
    assert_eq!(
        find_catalog_item(&items, &mission_validation::AssetId::from(known))
            .map(|i| i.display_name.as_str()),
        Some("AmmoBox 50cal 100rnd")
    );
    assert!(
        find_catalog_item(
            &items,
            &mission_validation::AssetId::new("{NOPE}Prefabs/Gone.et")
        )
        .is_none(),
        "an id that left the catalogue must resolve to None, not to a neighbour"
    );

    assert_eq!(
        find_catalog_item(&items, &mission_validation::AssetId::from(known))
            .and_then(placeable_palette),
        Some(CatalogPalette::Object)
    );
    for rejected in [
        "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd_base.et",
        "{DEADBEEFDEADBEEF}Prefabs/Props/Military/Unregistered.et",
    ] {
        assert_eq!(
            find_catalog_item(&items, &mission_validation::AssetId::from(rejected))
                .and_then(placeable_palette),
            None,
            "the Objects palette drops {rejected}, so a favourite must read it stale"
        );
    }

    let vehicles = vehicle_items();
    assert_eq!(
        find_catalog_item(
            &vehicles,
            &mission_validation::AssetId::new("{B}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_PKM.et")
        )
        .and_then(placeable_palette),
        Some(CatalogPalette::Vehicle)
    );
    for item in &vehicles {
        if item.kind == "vehicle" && item.r#abstract == Some(true) {
            assert_eq!(
                placeable_palette(item),
                None,
                "an abstract vehicle is not placeable: {}",
                item.resource_name
            );
        }
    }

    let chars = golden_items();
    fn leaf_ids(nodes: &[CatalogNode], out: &mut Vec<String>) {
        for n in nodes {
            if n.payload.is_some() {
                out.push(n.id.to_string());
            }
            leaf_ids(&n.children, out);
        }
    }
    let mut ids = Vec::new();
    leaf_ids(&build_catalog_tree(&chars, "BLUFOR"), &mut ids);
    assert!(!ids.is_empty(), "the golden must offer BLUFOR leaves");
    for id in &ids {
        assert_eq!(
            find_catalog_item(&chars, &mission_validation::AssetId::from(id.as_str()))
                .and_then(placeable_palette),
            Some(CatalogPalette::Character),
            "a Factions leaf must resolve placeable: {id}"
        );
    }
    assert!(build_catalog_tree(&chars, "OPFOR").is_empty());
    for id in &ids {
        assert!(
            find_catalog_item(&chars, &mission_validation::AssetId::from(id.as_str()))
                .and_then(placeable_palette)
                .is_some(),
            "the Eden side chip must not make a favourite stale: {id}"
        );
    }

    for item in chars.iter().filter(|i| i.kind.starts_with("gear")) {
        assert_eq!(placeable_palette(item), None);
    }
}
