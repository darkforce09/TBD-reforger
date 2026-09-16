//! Role: Module boundary for doc/store/tests.
//! Position: `doc/store/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;
use yrs::Transact;

fn ids_sorted(soa: &SlotSoa) -> Vec<String> {
    let mut v = soa.ids.clone();
    v.sort();
    v
}

fn row_of(soa: &SlotSoa, id: &str) -> usize {
    soa.ids.iter().position(|s| s == id).expect("id present")
}

fn slots_digest(soa: &SlotSoa) -> Vec<(String, u32, u32)> {
    let mut rows: Vec<(String, u32, u32)> = soa
        .ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            (
                id.clone(),
                soa.xy[i * 2].to_bits(),
                soa.xy[i * 2 + 1].to_bits(),
            )
        })
        .collect();
    rows.sort();
    rows
}

fn seeded_core() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.seed_random(8, 12800.0, 12800.0, 42);
    doc.set_origin_init(false);
    assert!(!doc.can_undo(), "the INIT seed must not be an undo step");
    doc
}

fn two_vehicles_three_slots() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    for id in ["s0", "s1", "s2"] {
        doc.add_slot(
            id, "sq", "L", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
        );
    }
    doc.add_vehicle(
        "v0",
        "Prefab/A.et",
        Some(1.0),
        Some(2.0),
        Some(0.0),
        Some(0.0),
    );
    doc.add_vehicle(
        "v1",
        "Prefab/B.et",
        Some(3.0),
        Some(4.0),
        Some(0.0),
        Some(0.0),
    );
    doc.set_origin_init(false);
    doc
}

fn crew_of(doc: &MissionDocCore, vehicle_id: &str) -> serde_json::Value {
    vehicles_of(doc)[vehicle_id]["crew"].clone()
}

#[cfg(feature = "scenario")]
fn hydrated_with_crew() -> MissionDocCore {
    let doc = two_vehicles_three_slots();
    doc.assign_crew_seat("v0", "driver", "s0");
    doc.assign_crew_seat("v0", "gunner", "s1");
    doc.assign_crew_seat("v1", "commander", "s2");
    let reloaded = save_and_reload(&doc);

    assert_eq!(
        crew_of(&reloaded, "v0")["driver"],
        "s0",
        "v0 driver hydrated"
    );
    assert_eq!(
        crew_of(&reloaded, "v0")["gunner"],
        "s1",
        "v0 gunner hydrated"
    );
    reloaded
}

use crate::source_scrub::strip_rust_lexical_noise;

fn orbat_fixture() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Layer", None);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "BLUFOR");
    doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
    doc.add_squad("sq-b", "faction-BLUFOR", "Bravo", None);
    doc
}

fn small_maps(doc: &MissionDocCore) -> serde_json::Value {
    serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
}

fn slots_map(doc: &MissionDocCore) -> serde_json::Value {
    serde_json::from_str(&doc.slots_json()).expect("slots_json")
}

fn vehicles_of(doc: &MissionDocCore) -> serde_json::Value {
    small_maps(doc)["vehiclesById"].clone()
}

#[cfg(feature = "scenario")]
fn save_and_reload(doc: &MissionDocCore) -> MissionDocCore {
    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&payload.to_string(), "lyr");
    reloaded
}

fn t220_lossy_payload() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 2,
        "map": {
            "terrain": "everon",
            "bounds": [100, 200, 300, 400],
            "center": [6400.5, 6400.25],
            "label": "ops-sector"
        },
        "environment": {},
        "editor": {
            "factions": [],
            "squads": [],

            "slots": [
                {
                    "id": "z-b", "squadId": "sq", "index": 0, "role": "Rifleman",
                    "stance": "stand",
                    "position": {
                        "x": 10.5, "y": 20.5, "z": 1.25, "rotation": 45.0,
                        "heading": 90.5, "source": "authored"
                    },
                    "customFlag": "keep-me",
                    "doctrineTag": "assault"
                },
                {
                    "id": "z-a", "squadId": "sq", "index": 1, "role": "Medic",
                    "stance": "stand",
                    "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 }
                },
                {
                    "id": "z-c", "squadId": "sq", "index": 2, "role": "SL",
                    "stance": "stand",
                    "position": { "x": 3.0, "y": 4.0, "z": 0.0, "rotation": 180.0 }
                }
            ],
            "editorLayers": []
        }
    })
}

fn briefing_fixture() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "US Army");
    doc.add_faction("faction-OPFOR", "OPFOR", "Soviet VDV");
    doc.add_squad("sq-a", "faction-BLUFOR", "1st", Some("Alpha".to_string()));
    doc.add_editor_layer("lyr", "Default Layer", None);
    doc.add_slot(
        "z1", "sq-a", "lyr", 0, "SL", None, None, 4839.2, 6620.8, 0.0, 270.0,
    );
    doc.set_origin_init(false);
    assert!(!doc.can_undo(), "the INIT seed must not be an undo step");
    doc
}

fn markers_of(doc: &MissionDocCore, faction_id: &str) -> Vec<serde_json::Value> {
    small_maps(doc)["factionsById"][faction_id]["briefing"]["markers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn marker_num(row: &serde_json::Value, key: &str) -> f64 {
    row[key]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} is a number: {row:?}"))
}

fn marker_rows(doc: &MissionDocCore) -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(&doc.briefing_marker_rows_json())
        .expect("the reader emits JSON")
        .as_array()
        .cloned()
        .expect("the reader emits an array")
}

const SITUATION: &str = "Enemy armour holds the east bank of the Levie crossing.\n\n\
                             Two T-72s were observed at 04:30, dug in north of the treeline.\n\
                             A third is unaccounted for.\n\n\
                             Civilians remain in the village. Weapons tight until contact.";

fn prose_of(doc: &MissionDocCore, faction_id: &str, key: &str) -> serde_json::Value {
    small_maps(doc)["factionsById"][faction_id]["briefing"][key].clone()
}

#[cfg(feature = "scenario")]
fn zones_fixture() -> MissionDocCore {
    let doc = MissionDocCore::new();

    doc.add_faction("f1", "BLUFOR", "US");
    doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.5, 0.0, 0.0,
    );
    doc.add_polygon_zone(
        "z_ao",
        "boundary",
        &[
            1000.25, -4210.75, 1600.5, -4210.75, 1600.5, -3800.125, 1000.25, -3800.125,
        ],
    );
    doc.set_zone_label("z_ao", Some("Area of Operations"));
    doc.set_zone_rules(
        "z_ao",
        Some(r#"{"graceSeconds":45.5,"penalty":"kill","warnEverySeconds":7.25}"#),
    );
    doc.add_circle_zone("z_obj", "objective_capture", 1234.5, -3990.25, 175.75);
    doc.set_zone_faction("z_obj", Some("blufor"));
    doc.set_zone_rules("z_obj", Some(r#"{"captureSeconds":180.5}"#));
    doc
}

#[cfg(feature = "scenario")]
fn wire_zones(payload: &serde_json::Value) -> Vec<serde_json::Value> {
    payload
        .get("zones")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

#[cfg(feature = "scenario")]
fn triggers_fixture() -> MissionDocCore {
    let doc = MissionDocCore::new();

    doc.add_faction("f1", "BLUFOR", "US");
    doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.5, 0.0, 0.0,
    );

    doc.add_polygon_trigger(
        "t_amb",
        "presence",
        &[
            1000.25, -4210.75, 1600.5, -4210.75, 1600.5, -3800.125, 1000.25, -3800.125,
        ],
    );
    doc.set_trigger_name("t_amb", Some("Ambush"));
    doc.set_trigger_owner("t_amb", Some("s1"));
    doc.set_trigger_rules(
        "t_amb",
        Some(r#"{"graceSeconds":45.5,"contestable":false}"#),
    );

    doc.add_circle_trigger("t_timer", "timer", 1234.5, -3990.25, 175.75);
    doc.set_trigger_rules("t_timer", Some(r#"{"announceEverySeconds":12.5}"#));
    doc
}

#[cfg(feature = "scenario")]
fn wire_triggers(payload: &serde_json::Value) -> Vec<serde_json::Value> {
    payload
        .get("triggers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

#[cfg(feature = "scenario")]
fn composition_row_json() -> String {
    serde_json::json!({
        "id": "c1",
        "title": "Fireteam + Technical",
        "author": "Sam",
        "category": "Infantry",
        "entities": [
            { "kind": "slot",    "dx":  -12.75, "dz": 8.5,   "rotation": 45.5,
              "role": "Squad Leader", "tag": "SL", "assetId": "Prefab/SL.et", "stance": "crouch",
              "loadout": { "gear": { "primary": "M4" } } },
            { "kind": "slot",    "dx":   12.25, "dz": -8.5,  "rotation": 0.0,
              "role": "Rifleman", "tag": "", "assetId": "Prefab/Rifleman.et", "stance": "stand" },
            { "kind": "vehicle", "dx":    0.5,  "dz": 30.125, "rotation": 270.75,
              "resourceName": "Prefab/Technical.et", "crewed": true,
              "crew": { "driver": "s0", "gunner": "s1" } },
            { "kind": "object",  "dx":  -30.5,  "dz": 0.25,  "rotation": 90.0,
              "alias": "sandbag_wall", "resourceName": "Prefab/Sandbag.et", "faction": "blufor" }
        ]
    })
    .to_string()
}

#[cfg(feature = "scenario")]
fn compositions_fixture() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.add_faction("f1", "BLUFOR", "US");
    doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.5, 0.0, 0.0,
    );
    doc.add_composition("c1", &composition_row_json());
    doc
}

#[cfg(feature = "scenario")]
fn wire_compositions(payload: &serde_json::Value) -> Vec<serde_json::Value> {
    payload
        .get("compositions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

#[cfg(feature = "scenario")]
fn canon(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.is_finite() => Value::from(f as i64),
            _ => v.clone(),
        },
        Value::Array(a) => Value::Array(a.iter().map(canon).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, val)| (k.clone(), canon(val))).collect())
        }
        _ => v.clone(),
    }
}

fn one_slot_one_layer() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("L", "Layer", None);
    doc.add_slot(
        "s0", "sq", "L", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.set_origin_init(false);
    doc
}

#[cfg(feature = "scenario")]
fn two_slots_visible_layer() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("L", "Layer", None);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
    doc.add_squad("sq", "faction-BLUFOR", "Alpha", Some("A1".into()));
    doc.add_slot("s0", "sq", "L", 0, "SL", None, None, 100.0, 200.0, 0.0, 0.0);
    doc.add_slot(
        "s1", "sq", "L", 1, "Rifleman", None, None, 110.0, 210.0, 0.0, 0.0,
    );
    doc.set_leader("sq", "s0");
    doc.set_origin_init(false);
    doc
}

#[cfg(feature = "scenario")]
fn template_payload_blufor_alpha() -> serde_json::Value {
    let src = MissionDocCore::new();
    src.set_origin_init(true);
    src.add_editor_layer("lyr", "Layer", None);
    src.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
    src.add_squad("sq-a", "faction-BLUFOR", "Alpha", Some("A1".into()));
    src.add_slot(
        "s0", "sq-a", "lyr", 0, "SL", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    src.add_slot(
        "s1", "sq-a", "lyr", 1, "Rifleman", None, None, 110.0, 210.0, 0.0, 0.0,
    );
    src.set_leader("sq-a", "s0");
    src.add_vehicle(
        "v0",
        "Prefab/Truck.et",
        Some(300.0),
        Some(400.0),
        Some(0.0),
        Some(0.0),
    );

    src.set_vehicle_faction("v0", "faction-BLUFOR");
    src.assign_crew_seat("v0", "driver", "s0");
    src.set_origin_init(false);
    crate::data::scenario::compile::compile_payload(
        &src.small_maps_json(),
        &src.slots_json(),
        false,
    )
}

fn id_array(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .expect("id array")
        .iter()
        .map(|s| s.as_str().expect("string id").to_string())
        .collect()
}

fn has_no_duplicates(ids: &[String]) -> bool {
    let mut seen = HashSet::new();
    ids.iter().all(|id| seen.insert(id.clone()))
}

#[cfg(feature = "scenario")]
fn doc_with_one_comment() -> (MissionDocCore, &'static str) {
    const TOKEN: &str = "CMT-TOKEN-ZZQ";
    let doc = two_slots_visible_layer();
    doc.add_comment(
        "c1",
        TOKEN,
        "tooltip body for CMT-TOKEN-ZZQ",
        1_234.5,
        6_789.5,
    );
    (doc, TOKEN)
}

#[cfg(feature = "scenario")]
fn doc_with_connectable_things() -> MissionDocCore {
    let doc = two_slots_visible_layer();
    doc.set_origin_init(true);
    doc.add_vehicle(
        "v0",
        "truck",
        Some(300.0),
        Some(400.0),
        Some(0.0),
        Some(0.0),
    );
    doc.add_entity("e0", "Crate", "crate_res", 500.0, 600.0, 0.0, 0.0);
    doc.set_origin_init(false);
    doc
}

fn two_sided_core(count: usize) -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("layer-1", "Layer 1", None);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "US Army");
    doc.add_faction("faction-OPFOR", "OPFOR", "Soviet Army");
    doc.add_squad("sq-blu", "faction-BLUFOR", "Alpha", None);
    doc.add_squad("sq-opf", "faction-OPFOR", "Bravo", None);
    for i in 0..count {
        let squad = if i % 2 == 0 { "sq-blu" } else { "sq-opf" };
        let index = u32::try_from(i / 2).expect("fixture index fits u32");
        doc.add_slot(
            &format!("n{i}"),
            squad,
            "layer-1",
            index,
            "Rifleman",
            None,
            None,
            f64::from(index),
            1.0,
            0.0,
            0.0,
        );
    }
    doc
}

fn soa_rows(s: &SlotSoa) -> Vec<String> {
    let SlotSoa {
        ids,
        xs,
        ys,
        xy,
        zs,
        rotations,
        stance,
        role_idx,
        tag_idx,
        squad_idx,
        layer_idx,
        side_keys,
        roles,
        tags,
        squads,
        layers,
    } = s.clone();
    let word = |dict: &[String], i: u32| match dict.get(i as usize) {
        Some(w) => w.clone(),
        None => format!("<{i}>"),
    };
    let mut rows: Vec<String> = (0..ids.len())
        .map(|r| {
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                ids[r],
                xs[r].to_bits(),
                ys[r].to_bits(),
                xy[r * 2].to_bits(),
                xy[r * 2 + 1].to_bits(),
                zs[r].to_bits(),
                rotations[r].to_bits(),
                stance[r],
                word(&roles, role_idx[r]),
                word(&tags, tag_idx[r]),
                word(&squads, squad_idx[r]),
                word(&layers, layer_idx[r]),
                side_keys[r],
            )
        })
        .collect();
    rows.sort();
    rows
}

fn only_fn_body(src: &str, marker: &str) -> String {
    let hits = src.matches(marker).count();
    assert_eq!(
        hits, 1,
        "expected exactly one `{marker}` in the scrubbed source, found {hits} — 0 means it was \
             renamed or deleted, 2+ means a shadow definition; either way this probe cannot examine \
             code it cannot unambiguously find"
    );
    let at = src.find(marker).expect("counted exactly one");
    let open = at + src[at..].find('{').expect("a fn has a body");
    let mut depth = 0usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return src[open..=open + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces after `{marker}`");
}

fn one_slot_awaiting_its_faction() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("layer-1", "Layer 1", None);
    doc.add_squad("sq-red", "faction-OPFOR", "Bravo", None);
    doc.add_slot(
        "s0", "sq-red", "layer-1", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc
}

fn keep_source_fixture() -> MissionDocCore {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Layer", None);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "US Army");
    doc.add_faction("faction-OPFOR", "OPFOR", "Soviet Army");
    doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
    doc.add_squad("sq-mid", "faction-BLUFOR", "Bravo", None);
    doc.add_squad("sq-c", "faction-BLUFOR", "Charlie", None);
    doc.add_squad("sq-opf", "faction-OPFOR", "Krasnyi", None);
    doc
}

fn squad_ids_of(doc: &MissionDocCore, faction_id: &str) -> Vec<String> {
    small_maps(doc)["factionsById"][faction_id]["squadIds"]
        .as_array()
        .expect("squadIds")
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect()
}

mod cases_1;
mod cases_2;
mod cases_3;
mod cases_4;
mod cases_5;
mod cases_6;
mod cases_7;
mod cases_8;
mod cases_9;
