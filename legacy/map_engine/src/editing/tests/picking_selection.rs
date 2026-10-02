//! Role: regression cases for the camera-unproject pick adapters.
//! Position: `editing/tests` in the map engine.
//! Signals & state: explicit frozen camera and document projections.
//! Invariants: square slot hits, circular vehicle hits, slot-first ties and marquee order.

use super::*;

#[test]
fn square_slots_circular_vehicles_and_equal_distance_policy() {
    let cam = mix_test_cam();
    let corner = mix_test_soa(&[("corner", 6400.75, 6400.75)]);
    let vehicles = vec![("corner-vehicle".into(), 6400.75, 6400.75)];
    assert_eq!(
        pick_slot(&cam, &corner, 400.0, 300.0).as_deref(),
        Some("corner")
    );
    assert_eq!(pick_vehicle(&cam, &vehicles, 400.0, 300.0), None);

    let ties = mix_test_soa(&[("first", 6400.0, 6400.0), ("second", 6400.0, 6400.0)]);
    let vehicles = vec![
        ("vehicle-first".into(), 6400.0, 6400.0),
        ("vehicle-second".into(), 6400.0, 6400.0),
    ];
    assert_eq!(
        pick_vehicle(&cam, &vehicles, 400.0, 300.0).as_deref(),
        Some("vehicle-first")
    );
    assert_eq!(
        pick_slot_or_vehicle(&cam, &ties, &vehicles, 400.0, 300.0).as_deref(),
        Some("first")
    );
    assert_eq!(
        marquee_slot_ids(&cam, &ties, 6399.0, 6399.0, 404.0, 296.0),
        vec!["first", "second"]
    );
    assert_eq!(
        marquee_vehicle_ids(&cam, &vehicles, 6399.0, 6399.0, 404.0, 296.0),
        vec!["vehicle-first", "vehicle-second"]
    );
}
/// T-491 Class-R — `pick_slot_or_vehicle`: only a slot in range → slot id.
#[test]
fn pick_slot_or_vehicle_slot_only() {
    let cam = mix_test_cam();
    let soa = mix_test_soa(&[("s0", 6400.0, 6400.0)]);
    let vehs: Vec<(String, f64, f64)> = vec![("v0".into(), 7000.0, 7000.0)]; // far
    let hit = pick_slot_or_vehicle(&cam, &soa, &vehs, 400.0, 300.0);
    assert_eq!(hit.as_deref(), Some("s0"));
}

/// T-491 Class-R — `pick_slot_or_vehicle`: only a vehicle in range → vehicle id.
#[test]
fn pick_slot_or_vehicle_vehicle_only() {
    let cam = mix_test_cam();
    let soa = mix_test_soa(&[("s0", 7000.0, 7000.0)]); // far
    let vehs: Vec<(String, f64, f64)> = vec![("v0".into(), 6400.0, 6400.0)];
    let hit = pick_slot_or_vehicle(&cam, &soa, &vehs, 400.0, 300.0);
    assert_eq!(hit.as_deref(), Some("v0"));
}

/// T-491 Class-R — when both are in the pick radius, the closer world-distance wins.
#[test]
fn pick_slot_or_vehicle_closer_wins() {
    let cam = mix_test_cam();
    // Slot at camera target; vehicle 0.2 m east — both inside ~1 m world pick radius @ zoom 2.
    let soa = mix_test_soa(&[("s0", 6400.0, 6400.0)]);
    let vehs: Vec<(String, f64, f64)> = vec![("v0".into(), 6400.2, 6400.0)];
    // Click the exact target → slot is closer (d=0).
    assert_eq!(
        pick_slot_or_vehicle(&cam, &soa, &vehs, 400.0, 300.0).as_deref(),
        Some("s0"),
        "exact center prefers the slot at the target"
    );
    // Pixel 1 px east of center: at scale=4, that is 0.25 m east — closer to the vehicle at +0.2 m.
    assert_eq!(
        pick_slot_or_vehicle(&cam, &soa, &vehs, 401.0, 300.0).as_deref(),
        Some("v0"),
        "offset toward the vehicle must pick the vehicle"
    );
}

/// T-491 Class-R — marquee returns slots first, then vehicles (append order).
#[test]
fn marquee_ids_with_vehicles_appends_vehicles_after_slots() {
    let cam = mix_test_cam();
    let soa = mix_test_soa(&[("s0", 6400.0, 6400.0), ("s1", 6410.0, 6410.0)]);
    let vehs: Vec<(String, f64, f64)> = vec![
        ("v0".into(), 6405.0, 6405.0),
        ("v_out".into(), 7000.0, 7000.0),
    ];
    // Press at world (6390,6390); release px that unprojects past (6420,6420).
    let start_wx = 6390.0;
    let start_wy = 6390.0;
    // At zoom 2 / scale 4: world +30 m ≈ +120 px from center(400,300) → (520, …).
    // flipY:false: screen +y is south in world? pan docs say screen +y ⇒ target north.
    // Safer: use cam.unproject to find a px, or form the box via known unproject of corners.
    // End px: unproject of (520, 180) — compute by inverting: we want world ~6420,6420.
    // Center is 6400,6400 at (400,300). Δworld (+20,+20); scale=4 → Δpx (+80, -80) if +y screen = -y world.
    let end_px = 400.0 + 20.0 * cam.scale();
    let end_py = 300.0 - 20.0 * cam.scale(); // screen up → world +y under flipY:false
    let ids = marquee_ids_with_vehicles(&cam, &soa, &vehs, start_wx, start_wy, end_px, end_py);
    assert!(
        ids.iter().any(|id| id == "s0") && ids.iter().any(|id| id == "s1"),
        "both slots in box: {ids:?}"
    );
    assert!(ids.iter().any(|id| id == "v0"), "vehicle in box: {ids:?}");
    assert!(
        !ids.iter().any(|id| id == "v_out"),
        "far vehicle excluded: {ids:?}"
    );
    let first_veh = ids
        .iter()
        .position(|id| id.starts_with('v'))
        .expect("a vehicle");
    let last_slot = ids
        .iter()
        .rposition(|id| id.starts_with('s'))
        .expect("a slot");
    assert!(
        last_slot < first_veh,
        "vehicles appended after slots: {ids:?}"
    );
}

/// Camera centred on Everon mid-map @ zoom 2 (scale = 4 px/m). Centre px (400,300) → (6400,6400).
fn mix_test_cam() -> crate::camera::ortho::state::OrthoCamera {
    let mut cam = crate::camera::ortho::state::OrthoCamera::new(800.0, 600.0, 6400.0, 6400.0, 2.0);
    cam.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
    cam
}

fn mix_test_soa(rows: &[(&str, f32, f32)]) -> SlotSoa {
    let mut soa = SlotSoa::default();
    for &(id, x, y) in rows {
        soa.ids.push(id.to_string());
        soa.xs.push(x);
        soa.ys.push(y);
        soa.xy.push(x);
        soa.xy.push(y);
    }
    soa
}

/// F4 — three placements refiled into one squad ⇒ 1 squad, 2 leader→member segments.
///
/// T-321 keeps the refile by forcing three squads the way the new model actually produces them:
/// renaming the bottom squad makes it authored, so the next placement starts a fresh one. That
/// exercises the mint-around-authoring branch and the original F4 render invariant at once.
#[test]
fn refile_merge_two_link_segments() {
    use crate::data::store::place_character_under_side;
    use crate::overlay::symbology::links::squad_links::build_squad_link_segments;
    use std::collections::HashMap;

    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Layer 1", None);
    let (_, s1, a) = place_character_under_side(
        &doc, "BLUFOR", "a", "lyr", "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
    )
    .expect("p1");
    doc.rename_squad(&s1, "Alpha");
    let (_, s2, b) = place_character_under_side(
        &doc, "BLUFOR", "b", "lyr", "Rifleman", None, None, 10.0, 0.0, 0.0, 0.0,
    )
    .expect("p2");
    doc.rename_squad(&s2, "Bravo");
    let (_, s3, c) = place_character_under_side(
        &doc, "BLUFOR", "c", "lyr", "Rifleman", None, None, 20.0, 0.0, 0.0, 0.0,
    )
    .expect("p3");
    assert_ne!(s1, s2, "a renamed squad is not grown");
    assert_ne!(s2, s3, "a renamed squad is not grown");

    doc.move_slot_to_squad(&b, &s1);
    doc.move_slot_to_squad(&c, &s1);

    let root: serde_json::Value =
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
    let squad_ids = root["factionsById"]["faction-BLUFOR"]["squadIds"]
        .as_array()
        .expect("squadIds");
    assert_eq!(squad_ids.len(), 1, "merged to one squad: {squad_ids:?}");
    assert_eq!(squad_ids[0], s1);
    assert!(root["squadsById"].get(&s2).is_none(), "s2 GC'd");
    assert!(root["squadsById"].get(&s3).is_none(), "s3 GC'd");

    let mut xy = HashMap::new();
    xy.insert(a.clone(), (0.0_f32, 0.0_f32));
    xy.insert(b.clone(), (10.0_f32, 0.0_f32));
    xy.insert(c.clone(), (20.0_f32, 0.0_f32));
    let verts = build_squad_link_segments(&squad_link_inputs(&doc), &xy);
    assert_eq!(
        verts.len() / 12,
        2,
        "size-3 squad ⇒ 2 segments; verts={}",
        verts.len()
    );
}
