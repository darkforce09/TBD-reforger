//! Role: Domain regression cases.
//! Position: `doc/operations/place_orbat/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn place_character_under_side_opfor() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let (fid, sid, slot) = place(&doc, "OPFOR", "n0").expect("place");
    assert_eq!(fid, "faction-OPFOR");
    assert_eq!(slot, "n0");
    assert_ne!(sid, "squad-1", "must not dump into squad-1");

    let root = small(&doc);
    let faction = &root["factionsById"]["faction-OPFOR"];
    assert_eq!(faction["key"], "OPFOR");
    assert_eq!(faction["name"], "OPFOR");
    let squad_ids = faction["squadIds"].as_array().expect("squadIds");
    assert_eq!(squad_ids.len(), 1);
    assert_eq!(squad_ids[0], sid);

    let squad = &root["squadsById"][&sid];
    let slot_ids = squad["slotIds"].as_array().expect("slotIds");
    assert_eq!(slot_ids, &vec![Value::String("n0".into())]);
    assert_eq!(squad["leaderSlotId"], "n0");
}

#[test]
fn slot_callsign_rank_roundtrip() {
    let doc = MissionDocCore::new();
    layer(&doc);
    place(&doc, "BLUFOR", "n1").expect("place");
    doc.update_slot_identity("n1", Some("Alpha-1".into()), Some("Sergeant".into()));
    let v = slots(&doc);
    assert_eq!(v["n1"]["callsign"], "Alpha-1");
    assert_eq!(v["n1"]["rank"], "Sergeant");

    doc.update_slot_identity("n1", None, None);
    let v = slots(&doc);
    assert!(v["n1"].get("callsign").is_none(), "{v}");
    assert!(v["n1"].get("rank").is_none(), "{v}");
}

#[test]
fn two_places_one_squad_same_side() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let (_, s1, a) = place(&doc, "BLUFOR", "a").expect("p1");
    let (_, s2, b) = place(&doc, "BLUFOR", "b").expect("p2");
    assert_eq!(s1, s2, "second placement joined the first's squad");

    let root = small(&doc);
    let squad_ids = root["factionsById"]["faction-BLUFOR"]["squadIds"]
        .as_array()
        .expect("squadIds");
    assert_eq!(squad_ids.len(), 1, "one squad, not one per click");
    assert_eq!(squad_ids[0], s1);

    let squad = &root["squadsById"][&s1];
    assert_eq!(
        squad["slotIds"].as_array().expect("slotIds"),
        &vec![Value::String(a.clone()), Value::String(b.clone())]
    );
    assert_eq!(squad["leaderSlotId"], a, "second place must not steal SL");
    let s = slots(&doc);
    assert_eq!(s[&a]["index"], 0);
    assert_eq!(s[&b]["index"], 1, "appended, not overwriting index 0");
}

#[test]
fn orbat_add_squad_increases_count_under_side() {
    let doc = MissionDocCore::new();
    layer(&doc);
    doc.add_faction("faction-OPFOR", "OPFOR", "OPFOR");
    let before = small(&doc)["factionsById"]["faction-OPFOR"]["squadIds"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    doc.add_squad("squad-OPFOR-1", "faction-OPFOR", "Squad 1", None);
    let after = small(&doc)["factionsById"]["faction-OPFOR"]["squadIds"]
        .as_array()
        .expect("squadIds")
        .len();
    assert_eq!(after, before + 1);
    assert!(small(&doc)["squadsById"].get("squad-OPFOR-1").is_some());
}

#[test]
fn orbat_add_role_increases_squad_slot_ids() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let (_, squad_id, first) = place_character_under_side(
        &doc, "BLUFOR", "n0", "lyr", "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
    )
    .expect("place");
    let before = small(&doc)["squadsById"][&squad_id]["slotIds"]
        .as_array()
        .expect("slotIds")
        .len();
    doc.add_slot(
        "n1",
        &squad_id,
        "lyr",
        before as u32,
        "Medic",
        Some("MED".into()),
        None,
        1.0,
        0.0,
        0.0,
        0.0,
    );
    let root = small(&doc);
    let ids = root["squadsById"][&squad_id]["slotIds"]
        .as_array()
        .expect("slotIds");
    assert_eq!(ids.len(), before + 1);
    assert!(ids.iter().any(|v| v == "n1"));
    assert_eq!(
        root["squadsById"][&squad_id]["leaderSlotId"], first,
        "add role must not steal SL"
    );

    assert_eq!(
        root["factionsById"]["faction-BLUFOR"]["squadIds"]
            .as_array()
            .expect("squadIds")
            .len(),
        1
    );
}

#[test]
fn place_rejects_invalid_side() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let before_small = small(&doc);
    let before_slots = slots(&doc);
    for bad in ["CIV", "nope"] {
        let err = place(&doc, bad, "x").expect_err("must reject");
        assert!(matches!(err, PlaceOrbatError::InvalidSide(_)), "{err:?}");
    }

    assert_eq!(small(&doc), before_small);
    assert_eq!(slots(&doc), before_slots);
    assert!(
        before_small["factionsById"]
            .as_object()
            .is_some_and(|m| m.is_empty()),
        "no faction minted on reject"
    );
    assert!(
        before_slots.as_object().is_some_and(|m| m.is_empty()),
        "no slot minted on reject"
    );
}

#[test]
fn five_places_build_one_squad_and_keep_every_body() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let mut ids = Vec::new();
    for i in 0..5 {
        let slot = format!("n{i}");
        place_at(&doc, "OPFOR", &slot, 1000.0 + 10.0 * f64::from(i), 2000.0);
        ids.push(slot);
    }
    doc.update_slot_identity("n3", Some("A-4".into()), Some("Corporal".into()));

    let squads = squad_ids(&doc, "OPFOR");
    assert_eq!(squads.len(), 1, "five clicks, one squad: {squads:?}");
    let squad = &small(&doc)["squadsById"][&squads[0]];
    assert_eq!(
        squad["slotIds"].as_array().expect("slotIds").len(),
        5,
        "all five bodies filed into it"
    );
    assert_eq!(squad["leaderSlotId"], "n0", "the first click leads");
    assert_eq!(squad["name"], "Squad 1");

    let s = slots(&doc);
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(s[id]["index"], i as u64, "dense index for {id}");
        assert_eq!(s[id]["position"]["x"], 1000.0 + 10.0 * i as f64, "{id} x");
        assert_eq!(s[id]["position"]["y"], 2000.0, "{id} y");
        assert_eq!(s[id]["squadId"], squads[0], "{id} squad");
    }
    assert_eq!(s["n3"]["callsign"], "A-4", "identity survives later places");
    assert_eq!(s["n3"]["rank"], "Corporal");
}

#[test]
fn each_side_keeps_its_own_current_squad() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let b1 = place_at(&doc, "BLUFOR", "b1", 1.0, 1.0);
    let o1 = place_at(&doc, "OPFOR", "o1", 2.0, 2.0);
    let b2 = place_at(&doc, "BLUFOR", "b2", 3.0, 3.0);
    assert_eq!(b1, b2, "BLUFOR kept its own squad across an OPFOR place");
    assert_ne!(b1, o1);
    assert_eq!(squad_ids(&doc, "BLUFOR").len(), 1);
    assert_eq!(squad_ids(&doc, "OPFOR").len(), 1);
}

#[test]
fn placement_starts_a_new_squad_rather_than_growing_an_authored_one() {
    type Author = fn(&MissionDocCore, &str) -> (String, Vec<Value>);
    let authors: [(&str, Author); 3] = [
        ("renamed", |doc, first| {
            doc.rename_squad(first, "Alpha");
            (first.to_string(), vec![Value::String("a".into())])
        }),
        ("vehicle", |doc, first| {
            doc.add_vehicle("veh-1", "{V}Truck.et", Some(1.0), Some(2.0), None, None);
            doc.attach_vehicle(first, "veh-1");
            (first.to_string(), vec![Value::String("a".into())])
        }),
        ("callsign", |doc, _first| {
            doc.add_squad(
                "squad-BLUFOR-cs",
                "faction-BLUFOR",
                "Squad 2",
                Some("A-1".into()),
            );
            ("squad-BLUFOR-cs".to_string(), Vec::new())
        }),
    ];

    for (label, author) in authors {
        let doc = MissionDocCore::new();
        layer(&doc);
        let first = place_at(&doc, "BLUFOR", "a", 10.0, 20.0);
        let (authored, roster) = author(&doc, &first);
        let before = squad_ids(&doc, "BLUFOR");
        assert_eq!(
            before.last(),
            Some(&authored),
            "{label}: authored is bottom"
        );

        let second = place_at(&doc, "BLUFOR", "b", 30.0, 40.0);
        assert_ne!(
            second, authored,
            "{label}: authored squad must not be grown"
        );
        let after = squad_ids(&doc, "BLUFOR");
        assert_eq!(
            after.len(),
            before.len() + 1,
            "{label}: exactly one new squad"
        );
        assert_eq!(after.last(), Some(&second), "{label}: minted at the bottom");

        assert_eq!(
            small(&doc)["squadsById"][&authored]["slotIds"]
                .as_array()
                .expect("slotIds"),
            &roster,
            "{label}: authored squad kept its roster"
        );

        let third = place_at(&doc, "BLUFOR", "c", 50.0, 60.0);
        let fourth = place_at(&doc, "BLUFOR", "d", 70.0, 80.0);
        assert_eq!(third, second, "{label}: accumulates in the new squad");
        assert_eq!(fourth, second, "{label}: accumulates in the new squad");
        assert_eq!(
            squad_ids(&doc, "BLUFOR").len(),
            before.len() + 1,
            "{label}: one squad per authoring event, never one per click"
        );
        assert_eq!(
            small(&doc)["squadsById"][&second]["leaderSlotId"],
            "b",
            "{label}: the new squad's first body leads it"
        );
    }
}

#[test]
fn an_empty_panel_minted_squad_is_filled_by_the_next_placement() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let first = place_at(&doc, "BLUFOR", "a", 10.0, 20.0);
    doc.rename_squad(&first, "Alpha");
    doc.add_squad("squad-BLUFOR-99", "faction-BLUFOR", "Squad 2", None);

    let target = place_at(&doc, "BLUFOR", "b", 30.0, 40.0);
    assert_eq!(target, "squad-BLUFOR-99", "filled the husk");
    assert_eq!(squad_ids(&doc, "BLUFOR").len(), 2, "no third squad");
    let squad = &small(&doc)["squadsById"]["squad-BLUFOR-99"];
    assert_eq!(squad["slotIds"].as_array().expect("slotIds").len(), 1);
    assert_eq!(
        squad["leaderSlotId"], "b",
        "first body into an empty squad leads"
    );
}

#[test]
fn the_current_squad_is_re_derived_after_undo() {
    let mut doc = MissionDocCore::new();
    layer(&doc);
    let s1 = place_at(&doc, "BLUFOR", "a", 10.0, 20.0);
    let depth_after_first = doc.undo_depth();
    let s2 = place_at(&doc, "BLUFOR", "b", 30.0, 40.0);
    assert_eq!(s1, s2);
    assert_eq!(
        doc.undo_depth() - depth_after_first,
        1,
        "an appending placement is one undo step"
    );

    assert!(doc.undo(), "undo the second placement");
    let root = small(&doc);
    assert_eq!(
        squad_ids(&doc, "BLUFOR"),
        vec![s1.clone()],
        "squad survives"
    );
    assert_eq!(
        root["squadsById"][&s1]["slotIds"]
            .as_array()
            .expect("slotIds"),
        &vec![Value::String("a".into())],
        "the undone body is gone, the first is not"
    );
    assert_eq!(root["squadsById"][&s1]["leaderSlotId"], "a", "SL intact");

    let s3 = place_at(&doc, "BLUFOR", "c", 50.0, 60.0);
    assert_eq!(s3, s1, "no stale pointer to an undone squad");
    assert_eq!(slots(&doc)["c"]["index"], 1);
    assert_eq!(squad_ids(&doc, "BLUFOR").len(), 1);
}

#[test]
fn minted_names_match_what_placement_writes() {
    for minted in ["Squad 1", "Squad 2", "Squad 17", "Squad 0", "", "  "] {
        assert!(is_minted_squad_name(minted), "{minted:?}");
    }
    for authored in ["Alpha", "Squad", "Squad A", "1st Squad", "Squad 1a"] {
        assert!(!is_minted_squad_name(authored), "{authored:?}");
    }

    let doc = MissionDocCore::new();
    layer(&doc);
    let first = place_at(&doc, "BLUFOR", "a", 1.0, 2.0);
    doc.rename_squad(&first, "Alpha");
    let second = place_at(&doc, "BLUFOR", "b", 3.0, 4.0);
    let root = small(&doc);
    assert_eq!(root["squadsById"][&second]["name"], "Squad 2", "ordinal");
    assert!(is_minted_squad_name(
        root["squadsById"][&second]["name"].as_str().unwrap_or("")
    ));
    assert!(is_open_for_placement(&root["squadsById"][&second]));
    assert!(!is_open_for_placement(&root["squadsById"][&first]));
}
