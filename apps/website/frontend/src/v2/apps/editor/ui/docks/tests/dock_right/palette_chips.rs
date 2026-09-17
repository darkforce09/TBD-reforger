use super::*;

/// T-646 (RIGHT-SUBMODE-001) — the Custom slot appears **only under Groups**. The predicate is
/// true for the Groups sub-mode and false for every other, and the tab→sub-mode map places the
/// Objects chip (Factions tab + objects_mode) into Objects — so flipping to Objects hides Custom
/// even though it is the same tab. Perturbation RED: were `custom_chip_visible` to admit any
/// non-Groups sub-mode, one of the `!` assertions below fails.
#[test]
fn custom_chip_only_under_groups() {
    assert!(
        custom_chip_visible(EdenSubmode::Groups),
        "Groups shows Custom"
    );
    assert!(!custom_chip_visible(EdenSubmode::Vehicles));
    assert!(!custom_chip_visible(EdenSubmode::Objects));
    assert!(!custom_chip_visible(EdenSubmode::Markers));
    assert!(!custom_chip_visible(EdenSubmode::Zones));
    // T-650 — the Compositions tab is not a Groups surface, so it hides Custom too.
    assert!(!custom_chip_visible(EdenSubmode::Compositions));
    // T-079 — nor is the Triggers tab a Groups surface.
    assert!(!custom_chip_visible(EdenSubmode::Triggers));
    // T-695 — nor is the Favourites tab (a persistent collection, not a place surface).
    assert!(!custom_chip_visible(EdenSubmode::Favourites));
    assert_eq!(EdenSubmode::from_tab(6, false), EdenSubmode::Favourites);
    assert_eq!(
        EdenSubmode::from_tab(6, true),
        EdenSubmode::Favourites,
        "the Objects chip splits the Factions tab alone — tab index wins"
    );

    // Tab → sub-mode: Factions (tab 0) is Groups unless the Objects chip is on.
    assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
    assert_eq!(EdenSubmode::from_tab(0, true), EdenSubmode::Objects);
    assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
    assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
    assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
    // T-650 — tab 4 is the Compositions surface.
    assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);
    // T-079 — tab 5 is the Triggers surface.
    assert_eq!(EdenSubmode::from_tab(5, false), EdenSubmode::Triggers);

    // The end-to-end visibility rule the render uses: Custom on the Factions tab iff not Objects,
    // and never on any other tab.
    assert!(
        custom_chip_visible(EdenSubmode::from_tab(0, false)),
        "Factions+side → Custom shown"
    );
    assert!(
        !custom_chip_visible(EdenSubmode::from_tab(0, true)),
        "Factions+Objects → Custom hidden"
    );
    for tab in [1usize, 2, 3, 4, 5, 6] {
        assert!(
            !custom_chip_visible(EdenSubmode::from_tab(tab, false)),
            "Custom hidden on tab {tab}"
        );
    }

    // The Custom slot is a SIXTH slot, distinct from the pinned 4-chip side row — it must not be
    // one of the side labels (that would fold it into the always-on row).
    assert_eq!(EDEN_CUSTOM_CHIP, "Custom");
    assert!(
        !EDEN_SIDE_CHIPS.iter().any(|c| *c == EDEN_CUSTOM_CHIP),
        "Custom is not a side chip"
    );
}

/// The tab was a one-line promise that placement would arrive in T-070, and the only vehicle
/// path was the ORBAT Manager's derived position. Both halves of the replacement are pinned:
/// the placeholder is gone, and a Vehicles leaf arms the **vehicle** place, not the character
/// one.
///
/// Source inspection, following `orbat_manager`'s precedent, because the thing under test is a
/// Leptos view whose handlers are `#[cfg(target_arch = "wasm32")]` — a native test cannot mount
/// it or fire the `pointerdown`. What it can do is fail loudly if the wiring is unpicked.
///
/// **Every needle is assembled at run time, and must stay that way.** This test searches the
/// file it is written in, so a needle spelled out contiguously — in an assertion, or in prose
/// *about* an assertion — puts itself into the haystack: absence checks can then never pass and
/// presence checks can never fail. That happened three times while writing this, and the third
/// was caught only by perturbation: a bare-symbol `contains` for the vehicle arm stayed GREEN
/// after the leaf was rewired to the character path, because the test's own literal satisfied
/// it. This program's signature defect in miniature — a check reporting success over an input
/// it never examined. The needle is therefore the whole call **expression**, which the file's
/// prose (which names the bare function) never contains.
#[test]
fn vehicles_tab_places_instead_of_promising() {
    const SRC: &str = DOCK_RIGHT_PRODUCTION_SOURCE;
    let stub = |what: &str, ticket: &str| format!("{what} placement {} {ticket}.", "lands in");
    let arm = |f: &str| format!("armed_placement::{f}{}", "(payload.clone())");

    assert!(
        !SRC.contains(&stub("Vehicle", "T-070")),
        "the Vehicles tab placeholder must be gone"
    );
    assert!(
        SRC.contains(&arm("begin_place_vehicle")),
        "a Vehicles leaf must arm the vehicle place path"
    );
    // T-215 pinned "the Markers tab is deliberately still a stub" here, so that the assertion
    // above proved THIS tab was the one that got wired rather than any tab being live.
    // **T-069 shipped the Markers tab**, so the pin is inverted rather than deleted — the stub
    // sentence is gone, and the two tabs still arm through DIFFERENT `editor_ops` entry points
    // (a marker is not a `/registry` leaf and carries no `PlacePayload`), which is what the
    // original was really asserting.
    assert!(
        !SRC.contains(&stub("Marker", "T-069")),
        "the Markers stub must be gone now that the tab is real"
    );
    assert!(
        SRC.contains(&format!("{}{}", "begin_place_marker", "(armed.clone())")),
        "a Markers icon row must arm the marker place path"
    );

    let ops = [
        crate::v2::core::test_support::editor_operations::ENTITY,
        crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
    ]
    .concat();
    assert!(
        ops.contains("pub fn begin_place_vehicle"),
        "editor_ops must expose the vehicle arm"
    );
    assert!(
        ops.contains("core.add_vehicle("),
        "the vehicle place must reach the core mutator"
    );
}

/// T-751 — T-695 kept `arm_favourite_place` textually distinct from `palette_rows` so T-215's
/// source-inspection needle (the palette's `begin_place_vehicle` + clone call expression) keeps
/// constraining the **palette** path specifically. That distinctness is the favourites arm
/// MOVING its payload rather than cloning. Pin both forms: the palette clone needle must
/// still exist exactly once, and the favourites move form (fn + `(payload),`) must exist and
/// must not grow a clone. Needles are fragment-assembled so this module is not its own haystack.
/// Note: `(payload)` is a prefix of the clone form, so the move needle ends at the trailing
/// comma that only the favourites match arm writes.
///
/// Wave-135 F3: Character + Object favourites arms are pinned the same way — vehicle-only left
/// Character/Object free to grow `.clone()` while this pin stayed green.
#[test]
fn favourites_place_arm_stays_clone_free() {
    const SRC: &str = DOCK_RIGHT_PRODUCTION_SOURCE;
    // Fragment the marker — a contiguous fn-name needle in this test would be a second hit.
    let marker = format!("{}{}", "fn arm_favourite_place", "(");
    let fav_arm = crate::v2::core::test_support::class_r_scrub::only_body(SRC, &marker);
    assert!(
        !fav_arm.contains(".clone()"),
        "T-751: arm_favourite_place must stay clone-free across Character/Object/Vehicle;              body was:\n{fav_arm}"
    );
    for (label, stem) in [
        ("Character", "begin_place"),
        ("Vehicle", "begin_place_vehicle"),
        ("Object", "begin_place_object"),
    ] {
        let palette = format!("{stem}{}", "(payload.clone())");
        let favourites = format!("{stem}{}", "(payload),");
        assert!(
            SRC.contains(&palette),
            "T-215/T-751 palette path must keep the {label} clone call expression"
        );
        assert_eq!(
            SRC.matches(&palette).count(),
            1,
            "T-751: exactly one palette-form {label} arm (clone); a second means favourites                  grew a clone"
        );
        assert!(
            SRC.contains(&favourites),
            "T-751: favourites {label} arm must MOVE the payload (trailing-comma form)"
        );
        assert_eq!(
            SRC.matches(&favourites).count(),
            1,
            "T-751: exactly one move-form {label} arm (the favourites match arm)"
        );
        assert!(
            fav_arm.contains(&format!("{stem}{}", "(payload)")),
            "T-751: arm_favourite_place {label} arm must call {stem}(payload)"
        );
    }
}

/// E1 + E5 — exact chip list; no CIV; no F-key labels in the chip row source of truth.
#[test]
fn eden_side_chips_labels_no_civ() {
    assert_eq!(EDEN_SIDE_CHIPS, &["BLUFOR", "OPFOR", "INDFOR", "Objects"]);
    assert_eq!(EDEN_SIDE_CHIPS.len(), 4);
    assert!(!EDEN_SIDE_CHIPS.iter().any(|c| *c == "CIV"));
    for label in EDEN_SIDE_CHIPS {
        assert!(
            !label.starts_with('F') || label == &"Objects",
            "F1–F6 mode row banned: {label}"
        );
        // F1…F6 are two-char labels like "F1" — none of our chips match.
        assert!(!matches!(*label, "F1" | "F2" | "F3" | "F4" | "F5" | "F6"));
    }
}

/// E2 — OPFOR chip writes the same side string `place_at` / EditorContext read.
#[test]
fn apply_eden_chip_opfor_sets_active_side() {
    let active_side = RwSignal::new(String::from("BLUFOR"));
    let objects_mode = RwSignal::new(true);
    apply_eden_chip(EdenChip::Opfor, active_side, objects_mode);
    assert_eq!(active_side.get_untracked(), "OPFOR");
    assert!(!objects_mode.get_untracked());
    assert!(eden_chip_selected(
        EdenChip::Opfor,
        &active_side.get_untracked(),
        objects_mode.get_untracked()
    ));
}

/// E3 — Objects chip flips objects_mode without clobbering side; coming-soon stub is gone.
#[test]
fn objects_chip_enables_mode_without_clobbering_side() {
    // T-254 — stub constant name must not remain (split so this assert's own source cannot
    // false-fail the contains check).
    let src = DOCK_RIGHT_PRODUCTION_SOURCE;
    let stub_const = ["OBJECTS_", "COMING_", "SOON"].concat();
    assert!(
        !src.contains(&stub_const),
        "Objects stub constant must be removed"
    );
    assert!(
        src.contains("begin_place_object") || src.contains("PaletteKind::Object"),
        "Objects palette must arm object places"
    );
    let active_side = RwSignal::new(String::from("OPFOR"));
    let objects_mode = RwSignal::new(false);
    apply_eden_chip(EdenChip::Objects, active_side, objects_mode);
    assert!(objects_mode.get_untracked());
    assert_eq!(
        active_side.get_untracked(),
        "OPFOR",
        "Objects must leave last side intact"
    );
    assert!(eden_chip_selected(
        EdenChip::Objects,
        &active_side.get_untracked(),
        objects_mode.get_untracked()
    ));
    assert!(!eden_chip_selected(
        EdenChip::Opfor,
        &active_side.get_untracked(),
        objects_mode.get_untracked()
    ));
}

/// T-255 — chip write + `build_catalog_tree(_, side)` is the dock rebuild contract: OPFOR
/// after a BLUFOR default must drop NATO leaves and keep only the USSR perturbation row.
#[test]
fn eden_chip_side_rebuilds_filtered_catalog() {
    use crate::v2::apps::editor::arsenal::asset_catalog::build_catalog_tree;
    use crate::v2::core::api::dto::RegistryResponse;

    let golden: RegistryResponse = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/api/GET__registry.json"
    )))
    .expect("golden");
    let mut items = golden.data;
    items.push(
        serde_json::from_value(serde_json::json!({
            "id": "ussr",
            "modpack_id": "mp",
            "resource_name": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
            "display_name": "USSR Rifleman",
            "category": "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
            "kind": "character",
            "sort_order": 99,
            "created_at": "2026-07-26T00:00:00Z",
            "updated_at": "2026-07-26T00:00:00Z",
        }))
        .expect("ussr row"),
    );

    let active_side = RwSignal::new(String::from("BLUFOR"));
    let objects_mode = RwSignal::new(false);
    let blufor_tree = build_catalog_tree(&items, &active_side.get_untracked());
    assert!(
        blufor_tree.iter().any(|n| n.id == "NATO"),
        "default BLUFOR chip keeps NATO"
    );

    apply_eden_chip(EdenChip::Opfor, active_side, objects_mode);
    let opfor_tree = build_catalog_tree(&items, &active_side.get_untracked());
    assert_eq!(active_side.get_untracked(), "OPFOR");
    assert_eq!(opfor_tree.len(), 1);
    assert!(
        !opfor_tree.iter().any(|n| n.id == "NATO"),
        "OPFOR chip must rebuild without NATO — got {:?}",
        opfor_tree.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()
    );
    fn has_leaf(
        nodes: &[crate::v2::apps::editor::arsenal::asset_catalog::CatalogNode],
        label: &str,
    ) -> bool {
        nodes
            .iter()
            .any(|n| (n.payload.is_some() && n.label == label) || has_leaf(&n.children, label))
    }
    assert!(
        has_leaf(&opfor_tree, "USSR Rifleman"),
        "OPFOR rebuild must keep USSR Rifleman"
    );
    assert!(!has_leaf(&opfor_tree, "US Rifleman"));
}
