//! Role: Module boundary for doc/operations/apply_faction/tests.
//! Position: `doc/operations/apply_faction/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use serde_json::json;

fn side_slot_count(doc: &MissionDocCore, side: &str) -> usize {
    let faction_id = format!("faction-{side}");
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return 0;
    };
    let squad_ids = root
        .get("factionsById")
        .and_then(|v| v.get(&faction_id))
        .and_then(|f| f.get("squadIds"))
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    let mut n = 0usize;
    for sid in squad_ids {
        let Some(sid) = sid.as_str() else {
            continue;
        };
        if let Some(arr) = root
            .get("squadsById")
            .and_then(|m| m.get(sid))
            .and_then(|sq| sq.get("slotIds"))
            .and_then(|a| a.as_array())
        {
            n += arr.len();
        }
    }
    n
}

fn layer(doc: &MissionDocCore) {
    doc.add_editor_layer("lyr", "Layer 1", None);
}

fn small(doc: &MissionDocCore) -> Value {
    serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
}

fn slots(doc: &MissionDocCore) -> Value {
    serde_json::from_str(&doc.slots_json()).expect("slots_json")
}

fn two_role_lib() -> FactionLibraryInput {
    FactionLibraryInput {
        name: "Soviet Army 1980s".into(),
        roles: vec![
            FactionLibraryRole {
                role: "Squad Leader".into(),
                tag: None,
                character: "{AAAA}Char.et".into(),
                loadout: Some(json!({
                    "version": 2,
                    "wear": {},
                    "weapons": [],
                    "summary": "AK-74"
                })),
            },
            FactionLibraryRole {
                role: "Rifleman".into(),
                tag: Some("AT".into()),
                character: "{BBBB}Rifleman.et".into(),
                loadout: None,
            },
        ],
        vehicles: vec![FactionLibraryVehicle {
            vehicle: "{CCCC}UAZ.et".into(),
            label: Some("UAZ-469".into()),
        }],
    }
}

fn seed_squads(doc: &MissionDocCore, side: &str, n: usize, slots_per: usize) -> Vec<String> {
    let faction_id = format!("faction-{side}");
    doc.add_faction(&faction_id, side, side);
    let mut ids = Vec::with_capacity(n);
    for s in 0..n {
        let sq = format!("squad-{side}-{s}");
        doc.add_squad(&sq, &faction_id, &format!("Squad {s}"), None);
        for k in 0..slots_per {
            let slot = format!("slot-{side}-{s}-{k}");
            doc.add_slot(
                &slot,
                &sq,
                "lyr",
                k as u32,
                "Rifleman",
                None,
                Some("{FFFF}Body.et".to_string()),
                1000.0 + 100.0 * s as f64,
                2000.0 + 15.0 * k as f64,
                0.0,
                0.0,
            );
            doc.update_slot_identity(
                &slot,
                Some(format!("A{s}-{k}")),
                Some("Corporal".to_string()),
            );
            if k == 0 {
                doc.set_leader(&sq, &slot);
            }
        }
        ids.push(sq);
    }
    ids
}

fn place(doc: &MissionDocCore, side: &str, n: usize, x: f64, y: f64) -> String {
    let slot_id = format!("slot-placed-{side}-{n}");
    crate::doc::place_character_under_side(
        doc,
        side,
        &slot_id,
        "lyr",
        "Rifleman",
        None,
        Some("{PLACED}Body.et".to_string()),
        x,
        y,
        0.0,
        0.0,
    )
    .expect("place");
    slot_id
}

fn side_squad_ids(doc: &MissionDocCore, side: &str) -> Vec<String> {
    faction_squad_ids(doc, &format!("faction-{side}"))
}

mod cases_1;
