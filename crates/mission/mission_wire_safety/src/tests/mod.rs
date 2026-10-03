//! Unit tests of the byte rule, the reporting caps and the cargo capacity walk.
//!
//! **Role:** holds the shared fixtures (a three-item catalog, a one-slot payload builder) and
//! mounts the case files.
//! **Position:** the crate's unit tests, reaching the public items through the crate root.
//! **Signals & state:** none; every case builds its own payload.
//! **Invariants:** the fixtures stay the inputs every case file reads.

use super::*;

use serde_json::{Value, json};

fn catalog_fixture() -> CargoPhysCatalog {
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
    c.insert(
        "pack_rn".into(),
        CargoPhys {
            display_name: "Rucksack".into(),
            max_weight_kg: Some(20.0),
            max_volume_cm3: Some(4000.0),
            ..CargoPhys::default()
        },
    );
    c
}

fn slot_with_cargo(wear: Value, cargo: Value) -> Value {
    json!({
        "editor": {
            "slots": [{
                "id": "s1",
                "role": "RFL",
                "loadout": { "version": 2, "wear": wear, "weapons": [], "cargo": cargo }
            }]
        }
    })
}

mod cases_1;
