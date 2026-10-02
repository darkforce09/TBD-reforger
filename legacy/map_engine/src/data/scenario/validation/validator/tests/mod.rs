//! Role: Module boundary for mission/validation/validator/tests.
//! Position: `mission/validation/validator/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use serde_json::json;

fn finding_for<'a>(findings: &'a [Finding], rule_id: &str) -> &'a Finding {
    findings
        .iter()
        .find(|f| f.rule_id == rule_id)
        .unwrap_or_else(|| panic!("expected a finding from {rule_id}, got {findings:?}"))
}

fn clean_orbat_payload() -> Value {
    json!({
        "schemaVersion": 1,
        "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
        "editor": {
            "factions": [
                {"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
                {"key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
            ],
            "squads": [
                {"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                 "slotIds": ["s1"], "leaderSlotId": "s1"},
                {"id": "sq2", "callsign": "Grom", "name": "Grom 1-1",
                 "slotIds": ["s2"], "leaderSlotId": "s2"}
            ],
            "slots": [
                {"id": "s1", "role": "SL", "position": {"x": 4839.2, "y": 6620.8, "z": 0.0}},
                {"id": "s2", "role": "RFL", "position": {"x": 6010.0, "y": 7211.5, "z": 0.0}}
            ]
        }
    })
}

use std::collections::HashSet;

fn ctx_with(ids: &[&str]) -> EvalContext {
    let set: HashSet<String> = ids.iter().map(|s| (*s).to_string()).collect();
    EvalContext::default().with_known_asset_ids(set)
}

use crate::data::scenario::wire_safety::{CargoPhys, CargoPhysCatalog};

fn ctx_min_mags(n: u64) -> EvalContext {
    EvalContext::default().with_loadout_policy(LoadoutPolicy::default().with_min_magazines(n))
}

fn cargo_catalog() -> CargoPhysCatalog {
    let mut c = CargoPhysCatalog::new();
    c.insert(
        "mag".into(),
        CargoPhys {
            display_name: "Mag".into(),
            weight_kg: Some(0.5),
            volume_cm3: Some(60.0),
            ..CargoPhys::default()
        },
    );
    c.insert(
        "vest_rn".into(),
        CargoPhys {
            display_name: "Plate Carrier".into(),
            max_weight_kg: Some(5.0),
            max_volume_cm3: Some(200.0),
            ..CargoPhys::default()
        },
    );
    c
}

mod cases_1;
mod cases_2;
