//! Role: Domain regression cases.
//! Position: `mission/compiler/kit/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn resolves_known_kits_and_faction_defaults() {
    let k = load_kit_aliases();
    assert_eq!(
        k.kit_for_resource(
            "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et"
        ),
        Some("kit:us_sl")
    );
    assert_eq!(k.kit_for_resource("unknown-resource"), None);
    assert_eq!(
        k.faction_default("blufor"),
        ("kit:us_rifleman", "preset:us_army_82nd")
    );
    assert_eq!(
        k.faction_default("opfor"),
        ("kit:sov_rifleman", "preset:sov_vdv")
    );

    assert_eq!(
        k.faction_default("mystery"),
        ("kit:us_rifleman", "preset:us_army_82nd")
    );
}

#[test]
fn resolves_known_vehicles_and_refuses_unknown() {
    let k = load_kit_aliases();
    assert_eq!(
        k.vehicle_for_resource("{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et"),
        Some("veh:m151_mg")
    );
    assert_eq!(
        k.vehicle_for_resource("{259EE7B78C51B624}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et"),
        Some("veh:uaz469")
    );
    assert_eq!(k.vehicle_for_resource("unknown-vehicle-prefab.et"), None);
}

#[test]
fn every_mod_faction_key_has_its_own_default() {
    let k = load_kit_aliases();
    assert_eq!(
        k.faction_default("indfor"),
        ("kit:fia_rifleman", "preset:fia")
    );
    assert_eq!(k.faction_default("civ"), ("kit:civ_generic", "preset:civ"));

    let blufor = k.faction_default("blufor");
    for side in ["opfor", "indfor", "civ"] {
        assert_ne!(
            k.faction_default(side),
            blufor,
            "{side} resolved to the blufor fallback — its factionDefaults row is missing"
        );
    }
}
