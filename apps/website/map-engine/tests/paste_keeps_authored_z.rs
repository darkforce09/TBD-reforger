//! Role: paste keeps authored z.
//! Position: `apps/website/map-engine/tests` in the map engine.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

#![cfg(feature = "store")]

use website_map_engine::data::store::MissionDocCore;

const ROOFTOP_Z: f64 = 37.3;

#[test]
fn a_pasted_slot_keeps_the_authored_z_of_the_slot_it_was_copied_from() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    doc.add_slot(
        "src", "sq1", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, ROOFTOP_Z, 90.0,
    );

    let rows: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    let clip = rows["src"].clone();
    let num = |k: &str| {
        clip["position"][k]
            .as_f64()
            .unwrap_or_else(|| panic!("clipboard row must carry position.{k}: {clip}"))
    };
    assert_eq!(
        num("z"),
        ROOFTOP_Z,
        "precondition: the clipboard row carries the authored z, so the paste has it in hand \
         without a second document read"
    );

    doc.paste_slots(
        vec!["copy".into()],
        vec!["sq1".into()],
        vec!["lyr".into()],
        vec![num("x")],
        vec![num("y")],
        vec![num("rotation")],
        vec![num("z")],
        vec!["Rifleman".into()],
        vec![String::new()],
        vec![String::new()],
        vec!["stand".into()],
        vec![String::new()],
        vec![String::new()],
        Some(400.0),
        Some(500.0),
        12800.0,
        12800.0,
    );

    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert_eq!(
        after["copy"]["position"]["z"].as_f64(),
        Some(ROOFTOP_Z),
        "the copy must land at the elevation it was copied from, exactly — no terrain-follow and \
         no f32 round trip. Slots after paste: {after}"
    );

    assert_eq!(after["src"]["position"]["z"].as_f64(), Some(ROOFTOP_Z));

    doc.add_slot(
        "flat", "sq1", "lyr", 1, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
    );
    doc.paste_slots(
        vec!["flat_copy".into()],
        vec!["sq1".into()],
        vec!["lyr".into()],
        vec![10.0],
        vec![20.0],
        vec![0.0],
        vec![0.0],
        vec!["Rifleman".into()],
        vec![String::new()],
        vec![String::new()],
        vec!["stand".into()],
        vec![String::new()],
        vec![String::new()],
        Some(50.0),
        Some(60.0),
        12800.0,
        12800.0,
    );
    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert_eq!(
        after["flat_copy"]["position"]["z"].as_f64(),
        Some(0.0),
        "carrying the source z through must not invent an elevation for a flat-map paste"
    );
}

#[test]
fn a_multi_slot_paste_gives_each_copy_its_own_source_elevation() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    let zs = vec![ROOFTOP_Z, 0.0, -4.75];
    doc.paste_slots(
        vec!["a".into(), "b".into(), "c".into()],
        vec!["sq1".into(); 3],
        vec!["lyr".into(); 3],
        vec![10.0, 20.0, 30.0],
        vec![10.0, 20.0, 30.0],
        vec![0.0, 0.0, 0.0],
        zs.clone(),
        vec!["Rifleman".into(); 3],
        vec![String::new(); 3],
        vec![String::new(); 3],
        vec!["stand".into(); 3],
        vec![String::new(); 3],
        vec![String::new(); 3],
        Some(100.0),
        Some(100.0),
        12800.0,
        12800.0,
    );
    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    for (id, z) in ["a", "b", "c"].iter().zip(zs) {
        assert_eq!(
            after[*id]["position"]["z"].as_f64(),
            Some(z),
            "slot {id} must get index-aligned z {z}; slots were: {after}"
        );
    }
}
