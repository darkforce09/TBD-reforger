//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn two_local_moves_are_two_undo_steps() {
    let mut doc = seeded_core();
    let d0 = slots_digest(&doc.materialize());

    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    let d1 = slots_digest(&doc.materialize());
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    let d2 = slots_digest(&doc.materialize());
    assert_ne!(d0, d1, "move 1 changed the doc");
    assert_ne!(d1, d2, "move 2 changed the doc");
    assert_eq!(doc.undo_depth(), 2, "two LOCAL txns = two stack items");

    assert!(doc.undo());
    assert_eq!(
        slots_digest(&doc.materialize()),
        d1,
        "undo 1 reverts ONLY move 2"
    );
    assert!(doc.can_undo(), "move 1 is still on the stack");

    assert!(doc.undo());
    assert_eq!(
        slots_digest(&doc.materialize()),
        d0,
        "undo 2 reverts move 1"
    );
    assert!(!doc.can_undo(), "the stack is now empty");
}

#[test]
fn mixed_slot_vehicle_two_calls_are_two_undo_steps() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(300.0),
        Some(400.0),
        Some(0.0),
        Some(0.0),
    );
    doc.set_origin_init(false);
    assert_eq!(doc.undo_depth(), 0);

    doc.move_entities(vec!["s0".to_string()], 10.0, 20.0, vec![0.0]);
    doc.move_vehicles(&["v0".to_string()], 10.0, 20.0);
    assert_eq!(
        doc.undo_depth(),
        2,
        "T-425 split path: two LOCAL txns for one mixed drag"
    );

    assert!(doc.undo());
    let vehs = vehicles_of(&doc);
    let slots = doc.materialize();
    let i = row_of(&slots, "s0");

    assert_eq!(
        slots.xs[i], 110.0,
        "slot still at post-drag x after one undo"
    );
    assert_eq!(
        slots.ys[i], 220.0,
        "slot still at post-drag y after one undo"
    );
    assert_eq!(vehs["v0"]["position"]["x"], 300.0, "vehicle undone");
    assert_eq!(vehs["v0"]["position"]["y"], 400.0, "vehicle undone");
    assert!(doc.can_undo(), "slot move still on the stack");
}

#[test]
fn mixed_slot_vehicle_atomic_move_is_one_undo_step() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(300.0),
        Some(400.0),
        Some(0.0),
        Some(45.0),
    );
    doc.set_origin_init(false);

    doc.move_entities_and_vehicles(
        vec!["s0".to_string()],
        &["v0".to_string()],
        10.0,
        20.0,
        vec![1.5],
    );
    assert_eq!(doc.undo_depth(), 1, "one mixed drag = one undo step");

    let slots = doc.materialize();
    let i = row_of(&slots, "s0");
    assert_eq!(slots.xs[i], 110.0);
    assert_eq!(slots.ys[i], 220.0);
    assert_eq!(slots.zs[i], 1.5, "slot z from zs[]");
    let vehs = vehicles_of(&doc);
    assert_eq!(vehs["v0"]["position"]["x"], 310.0);
    assert_eq!(vehs["v0"]["position"]["y"], 420.0);
    assert_eq!(vehs["v0"]["position"]["z"], 0.0, "vehicle z preserved");
    assert_eq!(
        vehs["v0"]["position"]["rotation"], 45.0,
        "vehicle rotation preserved"
    );

    assert!(doc.undo());
    assert_eq!(doc.undo_depth(), 0);
    let slots = doc.materialize();
    let i = row_of(&slots, "s0");
    assert_eq!(slots.xs[i], 100.0, "slot restored");
    assert_eq!(slots.ys[i], 200.0, "slot restored");
    let vehs = vehicles_of(&doc);
    assert_eq!(
        vehs["v0"]["position"]["x"], 300.0,
        "vehicle restored with slot"
    );
    assert_eq!(
        vehs["v0"]["position"]["y"], 400.0,
        "vehicle restored with slot"
    );
    assert_eq!(vehs["v0"]["position"]["rotation"], 45.0);
    assert!(!doc.can_undo());
}

#[test]
fn unmanned_place_split_calls_are_three_undo_steps() {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "BLUFOR");
    doc.set_origin_init(false);
    assert_eq!(doc.undo_depth(), 0);

    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(10.0),
        Some(20.0),
        Some(0.0),
        Some(0.0),
    );
    doc.set_vehicle_faction("v0", "faction-BLUFOR");
    doc.set_vehicle_crewed("v0", false);
    assert_eq!(
        doc.undo_depth(),
        3,
        "split place path: add + faction + crewed = three undo steps"
    );
    assert_eq!(vehicles_of(&doc)["v0"]["crewed"], false);
    assert_eq!(vehicles_of(&doc)["v0"]["factionId"], "faction-BLUFOR");
}

#[test]
fn place_vehicle_with_crew_stamp_is_one_undo_step_unmanned() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "BLUFOR");
    doc.set_origin_init(false);
    assert_eq!(doc.undo_depth(), 0);

    doc.place_vehicle_with_crew_stamp(
        "v0",
        "Prefab/Vehicle.et",
        10.0,
        20.0,
        0.0,
        0.0,
        "faction-BLUFOR",
        false,
    );
    assert_eq!(doc.undo_depth(), 1, "atomic stamp = one undo step");
    let v = &vehicles_of(&doc)["v0"];
    assert_eq!(v["position"]["x"], 10.0);
    assert_eq!(v["position"]["y"], 20.0);
    assert_eq!(v["factionId"], "faction-BLUFOR");
    assert_eq!(v["crewed"], false, "unmanned intent stamped");

    assert!(doc.undo());
    assert_eq!(doc.undo_depth(), 0);
    assert!(
        vehicles_of(&doc).get("v0").is_none(),
        "one undo removes the whole place"
    );
}

#[test]
fn place_vehicle_with_crew_stamp_omits_crewed_when_manned() {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_faction("faction-OPFOR", "OPFOR", "OPFOR");
    doc.set_origin_init(false);

    doc.place_vehicle_with_crew_stamp(
        "v1",
        "Prefab/Truck.et",
        1.0,
        2.0,
        0.0,
        90.0,
        "faction-OPFOR",
        true,
    );
    assert_eq!(doc.undo_depth(), 1);
    let v = &vehicles_of(&doc)["v1"];
    assert!(v["crewed"].is_null(), "manned = omit crewed key");
    assert_eq!(v["factionId"], "faction-OPFOR");
    assert_eq!(v["position"]["rotation"], 90.0);
}

#[test]
fn rotate_entities_is_one_undo_step_for_multi_selection() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.add_slot(
        "s1", "sq", "lyr", 1, "Rifleman", None, None, 110.0, 200.0, 0.0, 45.0,
    );
    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(300.0),
        Some(400.0),
        Some(1.5),
        Some(10.0),
    );
    doc.set_origin_init(false);
    assert_eq!(doc.undo_depth(), 0);

    let n = doc.rotate_entities(
        &[
            ("s0".into(), true, 90.0),
            ("s1".into(), true, 180.0),
            ("v0".into(), false, 270.0),
        ],
        8192.0,
        8192.0,
    );
    assert_eq!(n, 3, "all three patches applied");
    assert_eq!(doc.undo_depth(), 1, "N rotates = ONE undo step");

    let slots = doc.materialize();
    assert_eq!(slots.rotations[row_of(&slots, "s0")], 90.0);
    assert_eq!(slots.rotations[row_of(&slots, "s1")], 180.0);
    assert_eq!(vehicles_of(&doc)["v0"]["position"]["rotation"], 270.0);
    assert_eq!(
        vehicles_of(&doc)["v0"]["position"]["x"],
        300.0,
        "rotate leaves vehicle x"
    );
    assert_eq!(
        vehicles_of(&doc)["v0"]["position"]["z"],
        1.5,
        "rotate leaves vehicle z"
    );

    assert!(doc.undo());
    assert_eq!(doc.undo_depth(), 0);
    let slots = doc.materialize();
    assert_eq!(slots.rotations[row_of(&slots, "s0")], 0.0);
    assert_eq!(slots.rotations[row_of(&slots, "s1")], 45.0);
    assert_eq!(vehicles_of(&doc)["v0"]["position"]["rotation"], 10.0);
}

#[test]
fn update_entity_transforms_is_one_undo_step_for_mixed_batch() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 5.0, 0.0,
    );
    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(300.0),
        Some(400.0),
        Some(2.0),
        Some(45.0),
    );
    doc.set_origin_init(false);

    let n = doc.update_entity_transforms(
        &[
            EntityTransformPatch {
                id: "s0".into(),
                is_slot: true,
                x: Some(150.0),
                y: Some(250.0),
                z: Some(5.0),
                rotation: None,
            },
            EntityTransformPatch {
                id: "v0".into(),
                is_slot: false,
                x: Some(310.0),
                y: Some(410.0),
                z: Some(2.0),
                rotation: Some(45.0),
            },
        ],
        8192.0,
        8192.0,
    );
    assert_eq!(n, 2);
    assert_eq!(doc.undo_depth(), 1, "batch position write = one undo step");

    let slots = doc.materialize();
    let i = row_of(&slots, "s0");
    assert_eq!(slots.xs[i], 150.0);
    assert_eq!(slots.ys[i], 250.0);
    assert_eq!(slots.zs[i], 5.0, "authored z preserved");
    let v = &vehicles_of(&doc)["v0"];
    assert_eq!(v["position"]["x"], 310.0);
    assert_eq!(v["position"]["y"], 410.0);
    assert_eq!(v["position"]["z"], 2.0);
    assert_eq!(v["position"]["rotation"], 45.0);

    assert!(doc.undo());
    let slots = doc.materialize();
    let i = row_of(&slots, "s0");
    assert_eq!(slots.xs[i], 100.0);
    assert_eq!(slots.ys[i], 200.0);
    assert_eq!(vehicles_of(&doc)["v0"]["position"]["x"], 300.0);
}

#[test]
fn update_slots_attr_batch_is_one_undo_step_across_many_slots() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.add_slot(
        "s1", "sq", "lyr", 1, "Medic", None, None, 300.0, 400.0, 0.0, 0.0,
    );
    doc.add_slot(
        "s2", "sq", "lyr", 2, "Engineer", None, None, 500.0, 600.0, 0.0, 0.0,
    );
    doc.set_origin_init(false);

    let depth_before = doc.undo_depth();
    let ids = ["s0".to_string(), "s1".to_string(), "s2".to_string()];

    let n = doc.update_slots_attr_batch(
        &ids,
        true,
        Some("Squad Leader".to_string()),
        None,
        None,
        None,
        None,
    );
    assert_eq!(n, 3, "the batch fans the commit out to every id");
    assert_eq!(
        doc.undo_depth(),
        depth_before + 1,
        "F-26: a multi-slot identity apply is EXACTLY one undo step (was N before the batch)"
    );

    let role_of = |soa: &SlotSoa, id: &str| -> String {
        let row = row_of(soa, id);
        soa.roles[soa.role_idx[row] as usize].clone()
    };
    let slots = doc.materialize();
    for id in ["s0", "s1", "s2"] {
        assert_eq!(
            role_of(&slots, id),
            "Squad Leader",
            "every slot took the applied role"
        );
    }

    assert!(doc.undo());
    let slots = doc.materialize();
    assert_eq!(role_of(&slots, "s0"), "Rifleman");
    assert_eq!(role_of(&slots, "s1"), "Medic");
    assert_eq!(role_of(&slots, "s2"), "Engineer");
    assert_eq!(
        doc.undo_depth(),
        depth_before,
        "the single undo consumed the single step the batch created"
    );
}

#[test]
fn move_entities_and_vehicles_moves_every_slot_and_vehicle_in_one_txn() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.add_slot(
        "s1", "sq", "lyr", 1, "Rifleman", None, None, 500.0, 600.0, 0.0, 0.0,
    );
    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(300.0),
        Some(400.0),
        Some(7.0),
        Some(45.0),
    );
    doc.add_vehicle(
        "v1",
        "Prefab/Vehicle.et",
        Some(900.0),
        Some(800.0),
        Some(-3.0),
        Some(270.0),
    );

    doc.add_vehicle("v_orbat", "Prefab/Vehicle.et", None, None, None, None);
    doc.set_origin_init(false);
    assert_eq!(doc.undo_depth(), 0, "the INIT seed is not an undo step");

    let (dx, dy) = (10.0, -20.0);
    doc.move_entities_and_vehicles(
        vec!["s0".to_string(), "s1".to_string(), "s_absent".to_string()],
        &[
            "v0".to_string(),
            "v1".to_string(),
            "v_orbat".to_string(),
            "v_absent".to_string(),
        ],
        dx,
        dy,
        vec![1.5, 2.5, 3.5],
    );

    let slots = doc.materialize();
    for (id, x, y, z) in [("s0", 110.0, 180.0, 1.5), ("s1", 510.0, 580.0, 2.5)] {
        let i = row_of(&slots, id);
        assert_eq!(slots.xs[i], x, "{id}: x moved by dx");
        assert_eq!(slots.ys[i], y, "{id}: y moved by dy");
        assert_eq!(slots.zs[i], z, "{id}: z is zs[i], not zs[0]");
    }
    assert_eq!(slots.len(), 2, "an absent slot id must not mint a row");

    let vehs = vehicles_of(&doc);
    for (id, x, y, z, rot) in [
        ("v0", 310.0, 380.0, 7.0, 45.0),
        ("v1", 910.0, 780.0, -3.0, 270.0),
    ] {
        assert_eq!(vehs[id]["position"]["x"], x, "{id}: x moved by dx");
        assert_eq!(vehs[id]["position"]["y"], y, "{id}: y moved by dy");
        assert_eq!(vehs[id]["position"]["z"], z, "{id}: z preserved");
        assert_eq!(
            vehs[id]["position"]["rotation"], rot,
            "{id}: rotation preserved"
        );
    }
    assert!(
        vehs["v_orbat"]["position"].is_null(),
        "an unplaced vehicle must not be given a position by a drag"
    );
    assert!(
        vehs["v_absent"].is_null(),
        "an absent vehicle id must not mint a row"
    );

    assert_eq!(
        doc.undo_depth(),
        1,
        "one mixed drag must be ONE LOCAL txn (two ⇒ the T-425 two-Ctrl+Z defect)"
    );

    assert!(doc.undo());
    assert_eq!(doc.undo_depth(), 0, "the whole drag came off in one undo");
    let slots = doc.materialize();
    assert_eq!(slots.xs[row_of(&slots, "s0")], 100.0, "s0 restored");
    assert_eq!(slots.ys[row_of(&slots, "s1")], 600.0, "s1 restored");
    let vehs = vehicles_of(&doc);
    assert_eq!(
        vehs["v0"]["position"]["x"], 300.0,
        "v0 restored by the SAME undo as the slots"
    );
    assert_eq!(
        vehs["v1"]["position"]["y"], 800.0,
        "v1 restored by the SAME undo as the slots"
    );
    assert!(!doc.can_undo(), "nothing left on the stack");

    assert!(doc.redo());
    let slots = doc.materialize();
    assert_eq!(slots.xs[row_of(&slots, "s0")], 110.0, "s0 re-applied");
    let vehs = vehicles_of(&doc);
    assert_eq!(vehs["v1"]["position"]["x"], 910.0, "v1 re-applied");
}

#[test]
fn crew_assign_then_clear_rides_the_vehicle_row() {
    let doc = two_vehicles_three_slots();
    assert!(crew_of(&doc, "v0").is_null(), "no crew before any board");

    doc.assign_crew_seat("v0", "driver", "s0");
    assert_eq!(crew_of(&doc, "v0")["driver"], "s0", "board wrote the seat");

    doc.assign_crew_seat("v0", "gunner", "s1");
    assert_eq!(crew_of(&doc, "v0")["gunner"], "s1", "second seat coexists");
    assert_eq!(crew_of(&doc, "v0")["driver"], "s0", "first seat untouched");

    doc.clear_crew_seat("v0", "driver");
    assert!(
        crew_of(&doc, "v0")["driver"].is_null(),
        "unboard cleared the seat"
    );
    assert_eq!(crew_of(&doc, "v0")["gunner"], "s1", "other seat survives");

    doc.clear_crew_seat("v0", "gunner");
    assert!(
        crew_of(&doc, "v0").is_null(),
        "empty crew removes the key: {}",
        vehicles_of(&doc)["v0"]
    );
}

#[test]
fn crew_slot_occupies_one_seat_across_all_vehicles() {
    let doc = two_vehicles_three_slots();

    doc.assign_crew_seat("v0", "driver", "s0");
    doc.assign_crew_seat("v0", "gunner", "s1");

    doc.assign_crew_seat("v0", "commander", "s0");
    assert_eq!(crew_of(&doc, "v0")["commander"], "s0", "moved to commander");
    assert!(
        crew_of(&doc, "v0")["driver"].is_null(),
        "s0 no longer double-seated in its own vehicle"
    );

    doc.assign_crew_seat("v1", "driver", "s0");
    assert_eq!(crew_of(&doc, "v1")["driver"], "s0", "s0 now crews v1");
    assert!(
        crew_of(&doc, "v0")["commander"].is_null(),
        "s0 evicted from v0 — no soldier in two vehicles at once: {}",
        vehicles_of(&doc)["v0"]
    );

    assert_eq!(crew_of(&doc, "v0")["gunner"], "s1", "v1 board spared s1");

    doc.assign_crew_seat("v0", "driver", "s0");
    assert_eq!(crew_of(&doc, "v0")["driver"], "s0", "s0 back in v0");
    assert!(
        crew_of(&doc, "v1").is_null(),
        "s0 evicted from v1 (its only seat) — the crew key is gone: {}",
        vehicles_of(&doc)["v1"]
    );
}

#[test]
fn crew_board_is_one_undo_step() {
    let mut doc = two_vehicles_three_slots();
    assert_eq!(doc.undo_depth(), 0, "the INIT seed is not an undo step");

    doc.assign_crew_seat("v0", "driver", "s0");
    assert_eq!(doc.undo_depth(), 1, "one board is ONE LOCAL txn");
    assert_eq!(crew_of(&doc, "v0")["driver"], "s0");

    assert!(doc.undo(), "undo the board");
    assert_eq!(doc.undo_depth(), 0, "nothing left on the stack");
    assert!(
        crew_of(&doc, "v0").is_null(),
        "one undo vacated the seat: {}",
        vehicles_of(&doc)["v0"]
    );

    assert!(doc.redo(), "redo re-boards");
    assert_eq!(
        crew_of(&doc, "v0")["driver"],
        "s0",
        "redo restored the seat"
    );
}

#[test]
fn crew_assign_ignores_missing_vehicle_and_empty_ids() {
    let doc = two_vehicles_three_slots();
    doc.assign_crew_seat("nope", "driver", "s0");
    doc.assign_crew_seat("v0", "", "s0");
    doc.assign_crew_seat("v0", "driver", "");
    assert!(
        crew_of(&doc, "v0").is_null(),
        "no crew written by any no-op"
    );
    assert_eq!(doc.undo_depth(), 0, "a no-op is not an undo step");
}

#[test]
fn crewed_flag_is_written_only_when_unmanned() {
    let doc = two_vehicles_three_slots();
    assert!(
        vehicles_of(&doc)["v0"]["crewed"].is_null(),
        "a fresh vehicle carries no crewed key (with-crew default)"
    );

    doc.set_vehicle_crewed("v0", false);
    assert_eq!(
        vehicles_of(&doc)["v0"]["crewed"],
        false,
        "unmanned intent is stored"
    );

    doc.set_vehicle_crewed("v0", true);
    assert!(
        vehicles_of(&doc)["v0"]["crewed"].is_null(),
        "flipping back to manned removes the key, not stores true"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn crew_and_crewed_survive_save_and_reload() {
    let doc = two_vehicles_three_slots();
    doc.assign_crew_seat("v0", "driver", "s0");
    doc.assign_crew_seat("v0", "cargo2", "s1");
    doc.set_vehicle_crewed("v1", false);

    let reloaded = save_and_reload(&doc);
    assert_eq!(
        crew_of(&reloaded, "v0")["driver"],
        "s0",
        "seat assignment round-trips"
    );
    assert_eq!(crew_of(&reloaded, "v0")["cargo2"], "s1", "cargo seat too");
    assert_eq!(
        vehicles_of(&reloaded)["v1"]["crewed"],
        false,
        "unmanned intent round-trips"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn crew_unboard_after_hydrate_clears_the_seat() {
    let doc = hydrated_with_crew();
    doc.clear_crew_seat("v0", "driver");
    assert!(
        crew_of(&doc, "v0")["driver"].is_null(),
        "post-hydrate unboard must vacate the seat, not no-op: {}",
        vehicles_of(&doc)["v0"]
    );
    assert_eq!(
        crew_of(&doc, "v0")["gunner"],
        "s1",
        "the other loaded seat is untouched"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn crew_board_after_hydrate_keeps_existing_crew() {
    let doc = hydrated_with_crew();

    doc.add_slot(
        "s3", "sq", "L", 0, "Rifleman", None, None, 30.0, 40.0, 0.0, 0.0,
    );
    doc.assign_crew_seat("v0", "commander", "s3");
    assert_eq!(
        crew_of(&doc, "v0")["commander"],
        "s3",
        "the new seat was written"
    );
    assert_eq!(
        crew_of(&doc, "v0")["driver"],
        "s0",
        "the loaded driver SURVIVES the post-hydrate board (pre-fix: wiped): {}",
        vehicles_of(&doc)["v0"]
    );
    assert_eq!(
        crew_of(&doc, "v0")["gunner"],
        "s1",
        "the loaded gunner SURVIVES too"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn crew_eviction_after_hydrate_reaches_hydrated_crews() {
    let doc = hydrated_with_crew();

    doc.assign_crew_seat("v1", "driver", "s0");
    assert_eq!(crew_of(&doc, "v1")["driver"], "s0", "s0 now crews v1");
    assert!(
        crew_of(&doc, "v0")["driver"].is_null(),
        "s0 evicted from the HYDRATED v0 — no soldier in two vehicles at once: {}",
        vehicles_of(&doc)["v0"]
    );
    assert_eq!(
        crew_of(&doc, "v0")["gunner"],
        "s1",
        "eviction is surgical — v0's gunner is spared"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn crew_board_after_hydrate_round_trips_merged() {
    let doc = hydrated_with_crew();
    doc.add_slot(
        "s3", "sq", "L", 0, "Rifleman", None, None, 30.0, 40.0, 0.0, 0.0,
    );
    doc.assign_crew_seat("v0", "commander", "s3");

    let twice = save_and_reload(&doc);
    assert_eq!(
        crew_of(&twice, "v0")["driver"],
        "s0",
        "loaded driver present after the second round-trip"
    );
    assert_eq!(
        crew_of(&twice, "v0")["gunner"],
        "s1",
        "loaded gunner present after the second round-trip"
    );
    assert_eq!(
        crew_of(&twice, "v0")["commander"],
        "s3",
        "the post-hydrate board persisted through the second round-trip: {}",
        vehicles_of(&twice)["v0"]
    );
}

#[test]
fn rust_lexical_scrubber_eats_comments_and_literals_but_not_code() {
    let out = strip_rust_lexical_noise(
        "// core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);\nlet keep = 1;\n",
    );
    assert!(
        !out.contains("move_entities_and_vehicles"),
        "a line comment must not survive: {out:?}"
    );
    assert!(
        out.contains("let keep = 1;"),
        "live code must survive: {out:?}"
    );

    for (label, src) in [
        (
            "doc comment",
            "/// see move_entities_and_vehicles\nlet keep = 1;",
        ),
        ("inner doc", "//! move_entities_and_vehicles\nlet keep = 1;"),
        (
            "block comment",
            "/* move_entities_and_vehicles */ let keep = 1;",
        ),
        (
            "nested block",
            "/* outer /* move_entities_and_vehicles */ still */ let keep = 1;",
        ),
        (
            "string literal",
            "let s = \"move_entities_and_vehicles\"; let keep = 1;",
        ),
        (
            "escaped string",
            "let s = \"\\\"move_entities_and_vehicles\\\"\"; let keep = 1;",
        ),
        (
            "raw string",
            "let s = r\"move_entities_and_vehicles\"; let keep = 1;",
        ),
        (
            "hashed raw string",
            "let s = r##\"move_entities_and_vehicles \"# \"##; let keep = 1;",
        ),
        (
            "byte string",
            "let s = b\"move_entities_and_vehicles\"; let keep = 1;",
        ),
    ] {
        let out = strip_rust_lexical_noise(src);
        assert!(
            !out.contains("move_entities_and_vehicles"),
            "{label} must not survive scrubbing: {out:?}"
        );
        assert!(
            out.contains("let keep = 1;"),
            "{label}: live code lost: {out:?}"
        );
    }

    let life = strip_rust_lexical_noise("fn f<'a>(x: &'a str, c: char) { let q = '\\''; }");
    assert!(
        life.contains("fn f<'a>(x: &'a str, c: char)"),
        "lifetimes kept: {life:?}"
    );
    assert!(!life.contains("'\\''"), "the char literal went: {life:?}");

    let tricky = strip_rust_lexical_noise("let s = \"// /*\"; call_me();");
    assert!(
        tricky.contains("call_me();"),
        "string-borne `//` must not eat code: {tricky:?}"
    );

    let src = "// a\nlet keep = 1;\n";
    let out = strip_rust_lexical_noise(src);
    assert_eq!(
        out.chars().count(),
        src.chars().count(),
        "char count preserved"
    );
    assert_eq!(
        out.lines().count(),
        src.lines().count(),
        "newlines preserved: {out:?}"
    );
}

#[test]
fn mission_editor_move_commit_names_the_atomic_mix_api() {
    let select = strip_rust_lexical_noise(concat!(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/website/frontend/src/editor/tools/select_tool.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/website/frontend/src/editor/state/picking.rs"
        ))
    ));
    assert!(
        select.contains("MissionDocCore::pick_slot_or_vehicle("),
        "select_tool.rs has no `MissionDocCore::pick_slot_or_vehicle(` call token outside \
             comments/strings — the mixed pick was forked or deleted"
    );
    assert!(
        select.contains("MissionDocCore::marquee_ids_with_vehicles("),
        "select_tool.rs has no `MissionDocCore::marquee_ids_with_vehicles(` call token outside \
             comments/strings — the mixed marquee was forked or deleted"
    );

    let editor = strip_rust_lexical_noise(concat!(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/website/frontend/src/editor/mission_editor.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/website/frontend/src/editor/canvas/gestures.rs"
        ))
    ));

    let move_arms: Vec<&str> = editor
        .split("LG::Move")
        .skip(1)
        .map(|s| s.split("LG::").next().unwrap_or(s))
        .filter(|arm| arm.contains(".move_entities_and_vehicles("))
        .collect();
    assert!(
        move_arms.len() == 1,
        "expected exactly one LG::Move arm committing via `.move_entities_and_vehicles(` \
             (found {}) — the atomic mixed-move commit was forked, duplicated or deleted",
        move_arms.len()
    );
    let move_arm = move_arms[0];
    assert!(
        move_arm.contains(".move_entities_and_vehicles("),
        "the Move arm has no `.move_entities_and_vehicles(` call token outside comments/\
             strings — T-574's defect was that a bare `contains` here accepted a comment"
    );

    assert!(
        !move_arm.contains("core.move_entities("),
        "Move arm calls move_entities alone (two-txn defect)"
    );
    assert!(
        !move_arm.contains("editor_ops::move_vehicles"),
        "Move arm calls editor_ops::move_vehicles (second txn)"
    );
}
