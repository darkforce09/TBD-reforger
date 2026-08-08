//! **T-777** — a pasted slot keeps the elevation it was authored at.
//!
//! `editor_ops::paste_at_cursor` used to push a literal ground value into `paste_slots`' `zs`
//! column for every copied slot, justified in a comment as parity with the flat-map JS oracle.
//! The operator set that parity aside on 2026-08-08: it was a migration safety net, not a
//! contract. And the zero was FINAL rather than provisional — the JS oracle's caller re-sampled
//! the DEM and wrote the real elevation straight back, but nothing in the Leptos frontend does
//! (`terrainZ` did not survive the React deletion), so copying a rooftop entity dropped the copy
//! to the ground, silently, inside the paste's own undo step.
//!
//! **Why this test lives HERE and not beside the fix.** `apps/website/frontend/src/editor_ops.rs`
//! is `#![cfg(target_arch = "wasm32")]` and `map-engine-core`'s `doc` feature is a wasm32-only
//! dependency of `website-frontend`, so no native test in that crate can build a `MissionDocCore`
//! or call `paste_at_cursor`. The frontend half — that the paste path resolves each slot's z
//! through the shared `slot_z` reader instead of inventing a zero, and that the resolved vector
//! stays index-aligned with the minted ids — is a source pin in
//! `apps/website/frontend/src/attributes.rs`, beside its two wave-127 siblings. THIS file pins the
//! other half natively: that a non-zero authored z, carried across the seam, actually survives
//! into the document. Without it the frontend fix could be perfectly correct and still land every
//! copy on the ground, with both pins green.

#![cfg(feature = "doc")]

use map_engine_core::doc::MissionDocCore;

/// A rooftop elevation: exactly representable as an f64 and NOT as an f32, so a value that took a
/// detour through the materialized SoA (whose `zs` column is f32) would not compare equal here.
const ROOFTOP_Z: f64 = 37.3;

/// The full copy → paste round trip, at the seam where the elevation actually lands.
///
/// The source z is deliberately non-zero: a paste whose sources all sit at ground level produces
/// byte-identical output before and after the fix, so such a test cannot observe this defect at
/// all. The second half below pins that flat case explicitly, as the control.
#[test]
fn a_pasted_slot_keeps_the_authored_z_of_the_slot_it_was_copied_from() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    doc.add_slot(
        "src", "sq1", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, ROOFTOP_Z, 90.0,
    );

    // `editor_ops::copy_selection` files the RAW `slots_json()` rows on the clipboard — this is
    // the exact value the paste path reads its z back out of, not a reconstruction of it.
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
        vec![num("z")], // T-777 — the carried elevation, was a hard-coded ground value
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
    // The original is untouched: paste adds, it does not move what it copied.
    assert_eq!(after["src"]["position"]["z"].as_f64(), Some(ROOFTOP_Z));

    // CONTROL — a genuinely ground-level source still pastes at ground level. This is the case the
    // pre-fix code produced for EVERY source, which is why the assertion above is the one that can
    // tell the two versions apart.
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

/// A multi-slot paste must not hand one entity another's elevation.
///
/// The frontend builds `zs` in the same walk that mints `ids`, so the correspondence is structural
/// — but `paste_slots` is where a mis-zipped pair would actually be written, and a swapped pair
/// looks green to any test that only checks "some slot has z = 37.3". Distinct z per slot, each
/// asserted against its OWN id.
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
