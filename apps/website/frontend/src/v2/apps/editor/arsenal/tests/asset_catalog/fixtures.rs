use super::*;
use crate::v2::core::api::dto::RegistryResponse;

/// The same committed golden the R-api gate pins (`dto::r_api`), so this test and the live
/// palette read byte-identical data.
const GOLDEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/api/GET__registry.json"
));

pub(super) fn golden_items() -> Vec<RegistryItem> {
    serde_json::from_str::<RegistryResponse>(GOLDEN)
        .expect("golden deserializes")
        .data
}

pub(super) fn character_row(resource: &str, name: &str, category: &str) -> RegistryItem {
    serde_json::from_value(serde_json::json!({
        "id": resource,
        "modpack_id": "mp",
        "resource_name": resource,
        "display_name": name,
        "category": category,
        "kind": "character",
        "sort_order": 0,
        "created_at": "2026-07-26T00:00:00Z",
        "updated_at": "2026-07-26T00:00:00Z",
    }))
    .expect("character row deserializes")
}

pub(super) fn leaf_labels(nodes: &[CatalogNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[CatalogNode], out: &mut Vec<String>) {
        for n in nodes {
            if n.payload.is_some() {
                out.push(n.label.clone());
            }
            walk(&n.children, out);
        }
    }
    walk(nodes, &mut out);
    out
}

/// T-255 Class-R — mixed BLUFOR+OPFOR rows: each chip sees only its side. Perturbation RED:
/// dropping the side filter (kind-only) would put USSR under BLUFOR and NATO under OPFOR.

// ── T-215 — the Vehicles palette ────────────────────────────────────────────────────────────

/// A registry row shaped like the live `/registry` vehicle rows (the golden fixture is the
/// 21-row character/gear capture, so it holds none).
pub(super) fn vehicle_row(
    resource: &str,
    name: &str,
    category: &str,
    is_abstract: bool,
) -> RegistryItem {
    serde_json::from_value(serde_json::json!({
        "id": resource,
        "modpack_id": "mp",
        "resource_name": resource,
        "display_name": name,
        "category": category,
        "kind": "vehicle",
        "abstract": is_abstract,
        "sort_order": 0,
        "created_at": "2026-07-26T00:00:00Z",
        "updated_at": "2026-07-26T00:00:00Z",
    }))
    .expect("vehicle row deserializes")
}

pub(super) fn vehicle_items() -> Vec<RegistryItem> {
    let mut v = golden_items(); // 8 characters + 13 gear — none may reach this tree
    v.push(vehicle_row(
        "{A}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et",
        "UAZ469",
        "ArmaReforger/Vehicles/Wheeled/UAZ469",
        false,
    ));
    v.push(vehicle_row(
        "{B}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_PKM.et",
        "UAZ469 PKM",
        "ArmaReforger/Vehicles/Wheeled/UAZ469",
        false,
    ));
    v.push(vehicle_row(
        "{C}Prefabs/Vehicles/Helicopters/Mi8MT/Mi8MT_base.et",
        "Mi8MT base",
        "ArmaReforger/Vehicles/Helicopters/Mi8MT",
        true, // abstract — a template, not placeable
    ));
    v
}

/// The whole category path becomes folders (not path-minus-last), so two variants of one family
/// stay under that family instead of collapsing into its parent.

// ── T-809 (F-22) — the merged Factions tree: one tree per faction ──────────────────────────────

/// A merged fixture in the SHAPE the live dev seed uses (`registry_dev.sql`, T-800): BLUFOR
/// characters and BLUFOR vehicles both rooted `NATO/US_Army/…`, plus an OPFOR character so the
/// side filter has something to exclude, plus an abstract vehicle template that must be dropped.
pub(super) fn merged_items() -> Vec<RegistryItem> {
    vec![
        character_row(
            "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
            "US Rifleman",
            "NATO/US_Army/Rifleman",
        ),
        character_row(
            "{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et",
            "US Medic",
            "NATO/US_Army/Medic",
        ),
        character_row(
            "{1111111111111111}Prefabs/Characters/Factions/OPFOR/USSR/Character_RU_Rifleman.et",
            "USSR Rifleman",
            "USSR/Motorized/Rifleman",
        ),
        vehicle_row(
            "{86B7B7522A75FF8B}Prefabs/Vehicles/Wheeled/M998/M1025_M2.et",
            "M1025 Humvee (M2)",
            "NATO/US_Army/Vehicles",
            false,
        ),
        vehicle_row(
            "{4D4D74A0BE9E8C1F}Prefabs/Vehicles/Tracked/M113/M113_M2.et",
            "M113 APC (M2)",
            "NATO/US_Army/Vehicles",
            false,
        ),
        vehicle_row(
            "{9999999999999999}Prefabs/Vehicles/Wheeled/M998/M998_base.et",
            "M998 base",
            "NATO/US_Army/Vehicles",
            true, // abstract template — must not reach a leaf
        ),
    ]
}

/// THE ACCEPTANCE this ticket exists for: a vehicle leaf is reachable **inside its faction**, in
/// the same tree as the characters — Eden's F1-Objects shape, not TBD's three-tab split. Under
/// `NATO` sit both the roles and a `Vehicles` sub-folder whose leaves are the placeable vehicles.

pub(super) fn object_row(
    resource: &str,
    name: &str,
    category: &str,
    kind: &str,
    is_abstract: bool,
) -> RegistryItem {
    serde_json::from_value(serde_json::json!({
        "id": resource,
        "modpack_id": "mp",
        "resource_name": resource,
        "display_name": name,
        "category": category,
        "kind": kind,
        "abstract": is_abstract,
        "sort_order": 0,
        "created_at": "2026-07-26T00:00:00Z",
        "updated_at": "2026-07-26T00:00:00Z",
    }))
    .expect("object row deserializes")
}

pub(super) fn object_items() -> Vec<RegistryItem> {
    let mut v = golden_items();
    // Registered in mod Data/registry.json (T-439) — must reach Objects leaves.
    v.push(object_row(
        "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd.et",
        "AmmoBox 50cal 100rnd",
        "ArmaReforger/Props/Military/AmmoBoxes",
        "crate",
        false,
    ));
    // Abstract — excluded regardless of registry.
    v.push(object_row(
        "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd_base.et",
        "AmmoBox 50cal base",
        "ArmaReforger/Props/Military/AmmoBoxes",
        "crate",
        true,
    ));
    // Registered composition-path crate.
    v.push(object_row(
            "{3568138FF7A659A1}Prefabs/Compositions/Misc/CustomEntities/InteractionPoints/AmmoBoxArsenal_Equipment_US_Apparel.et",
            "AmmoBoxArsenal Equipment US Apparel",
            "ArmaReforger/Compositions/Misc/CustomEntities/InteractionPoints",
            "crate",
            false,
        ));
    // Synthesises prop:unregistered_test_crate — NOT in mod registry → dropped (T-439).
    v.push(object_row(
        "{DEADBEEFDEADBEEF}Prefabs/Props/Military/Unregistered.et",
        "Unregistered Test Crate",
        "ArmaReforger/Props/Military",
        "crate",
        false,
    ));
    v
}
