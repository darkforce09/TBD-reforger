//! Role: Domain regression cases.
//! Position: `doc/operations/apply_faction/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn apply_faction_library_counts() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let lib = two_role_lib();
    let r = apply_faction_library(&doc, "OPFOR", "lyr", &lib).expect("apply");
    assert_eq!(r.roles_applied, 2);
    assert_eq!(side_slot_count(&doc, "OPFOR"), 2);
    let root = small(&doc);
    let squad_ids = root["factionsById"]["faction-OPFOR"]["squadIds"]
        .as_array()
        .expect("squadIds");
    assert!(!squad_ids.is_empty());
    assert_eq!(squad_ids.len(), 1);
    assert_eq!(root["squadsById"][&r.squad_id]["name"], "Soviet Army 1980s");
}

#[test]
fn apply_faction_sets_leader() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let mut lib = two_role_lib();

    lib.roles.swap(0, 1);
    let r = apply_faction_library(&doc, "BLUFOR", "lyr", &lib).expect("apply");
    let root = small(&doc);
    assert_eq!(
        root["squadsById"][&r.squad_id]["leaderSlotId"],
        "slot-BLUFOR-apply-1"
    );
    assert_eq!(r.leader_slot_id, "slot-BLUFOR-apply-1");

    let lib2 = FactionLibraryInput {
        name: "Alpha".into(),
        roles: vec![
            FactionLibraryRole {
                role: "Rifleman".into(),
                tag: None,
                character: "c1".into(),
                loadout: None,
            },
            FactionLibraryRole {
                role: "Medic".into(),
                tag: None,
                character: "c2".into(),
                loadout: None,
            },
        ],
        vehicles: vec![],
    };
    let r2 = apply_faction_library(&doc, "INDFOR", "lyr", &lib2).expect("apply2");
    assert_eq!(r2.leader_slot_id, "slot-INDFOR-apply-0");
}

#[test]
fn apply_faction_copies_loadout() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let lib = two_role_lib();
    let r = apply_faction_library(&doc, "OPFOR", "lyr", &lib).expect("apply");
    let s = slots(&doc);
    let lo = &s["slot-OPFOR-apply-0"]["loadout"];
    assert_eq!(lo["summary"], "AK-74");
    assert_eq!(lo["version"], 2);

    assert!(s["slot-OPFOR-apply-1"].get("loadout").is_none());
    assert_eq!(r.roles_applied, 2);
}

#[test]
fn apply_faction_vehicles() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let lib = two_role_lib();
    let r = apply_faction_library(&doc, "OPFOR", "lyr", &lib).expect("apply");
    assert_eq!(r.vehicles_applied, 1);
    let root = small(&doc);
    let vids = root["squadsById"][&r.squad_id]["vehicleIds"]
        .as_array()
        .expect("vehicleIds");
    assert_eq!(vids.len(), 1);
    assert!(root["vehiclesById"].get("veh-OPFOR-apply-0").is_some());
    assert_eq!(
        root["vehiclesById"]["veh-OPFOR-apply-0"]["resourceName"],
        "{CCCC}UAZ.et"
    );
    assert!(
        root["vehiclesById"]["veh-OPFOR-apply-0"]
            .get("position")
            .is_some()
    );
    let xy = doc.vehicle_xy_flat();
    assert_eq!(xy.len(), 2);
}

#[test]
fn reapply_keeps_overlapping_slot_ids_and_positions() {
    let doc = MissionDocCore::new();
    layer(&doc);
    apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("first");

    doc.set_slot_position("slot-BLUFOR-apply-0", 111.0, 222.0, 0.0, 0.0);

    let lib2 = FactionLibraryInput {
        name: "Soviet Army 1980s v2".into(),
        roles: vec![
            FactionLibraryRole {
                role: "Platoon Leader".into(),
                tag: None,
                character: "{DDDD}PL.et".into(),
                loadout: None,
            },
            FactionLibraryRole {
                role: "Squad Leader".into(),
                tag: None,
                character: "{AAAA}Char.et".into(),
                loadout: None,
            },
            FactionLibraryRole {
                role: "Machinegunner".into(),
                tag: Some("MG".into()),
                character: "{EEEE}MG.et".into(),
                loadout: None,
            },
        ],
        vehicles: vec![],
    };
    let r2 = apply_faction_library(&doc, "BLUFOR", "lyr", &lib2).expect("second");
    assert_eq!(r2.roles_applied, 3);
    assert_eq!(side_slot_count(&doc, "BLUFOR"), 3);

    let s = slots(&doc);

    assert_eq!(s["slot-BLUFOR-apply-0"]["role"], "Platoon Leader");
    assert_eq!(s["slot-BLUFOR-apply-1"]["role"], "Squad Leader");
    assert_eq!(s["slot-BLUFOR-apply-2"]["role"], "Machinegunner");

    assert_eq!(s["slot-BLUFOR-apply-0"]["position"]["x"], 111.0);
    assert_eq!(s["slot-BLUFOR-apply-0"]["position"]["y"], 222.0);

    assert!(s["slot-BLUFOR-apply-0"].get("loadout").is_none());

    assert_eq!(r2.leader_slot_id, "slot-BLUFOR-apply-1");
}

#[test]
fn apply_faction_replace_not_merge() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let lib2 = two_role_lib();
    apply_faction_library(&doc, "BLUFOR", "lyr", &lib2).expect("first");
    assert_eq!(side_slot_count(&doc, "BLUFOR"), 2);

    let lib1 = FactionLibraryInput {
        name: "Solo".into(),
        roles: vec![FactionLibraryRole {
            role: "Rifleman".into(),
            tag: None,
            character: "c".into(),
            loadout: None,
        }],
        vehicles: vec![],
    };
    apply_faction_library(&doc, "BLUFOR", "lyr", &lib1).expect("second");
    assert_eq!(side_slot_count(&doc, "BLUFOR"), 1);
    let root = small(&doc);
    assert_eq!(
        root["factionsById"]["faction-BLUFOR"]["squadIds"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
        1
    );

    assert!(
        root["vehiclesById"]
            .as_object()
            .map(|m| m.is_empty())
            .unwrap_or(true)
    );
}

#[test]
fn apply_rejects_civ() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let err = apply_faction_library(&doc, "CIV", "lyr", &two_role_lib()).expect_err("civ");
    assert!(matches!(err, ApplyFactionError::InvalidSide(_)));
}

#[test]
fn apply_refuses_to_collapse_squads_and_writes_nothing() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let seeded = seed_squads(&doc, "OPFOR", 3, 2);
    let small_before = small(&doc);
    let slots_before = slots(&doc);
    assert_eq!(side_slot_count(&doc, "OPFOR"), 6);

    let err = apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect_err("refuse");
    match &err {
        ApplyFactionError::WouldCollapseSquads {
            side,
            squad_ids,
            blocking,
            slots_at_risk,
        } => {
            assert_eq!(side, "OPFOR");
            assert_eq!(squad_ids, &seeded);

            assert_eq!(blocking.len(), 2);
            assert_eq!(blocking[0].id, "squad-OPFOR-1");
            assert_eq!(blocking[1].id, "squad-OPFOR-2");

            assert!(blocking.iter().all(|b| b.why == "2 slots"), "{blocking:?}");

            assert_eq!(*slots_at_risk, 4);
        }
        other => panic!("expected WouldCollapseSquads, got {other:?}"),
    }

    let msg = err.to_string();
    assert!(msg.contains("OPFOR"), "{msg}");
    assert!(msg.contains("3 squads"), "{msg}");
    assert!(msg.contains("Nothing was changed"), "{msg}");
    assert!(msg.contains("\"Squad 1\" (2 slots)"), "{msg}");
    assert!(msg.contains("\"Squad 2\" (2 slots)"), "{msg}");

    assert_eq!(small(&doc), small_before);
    assert_eq!(slots(&doc), slots_before);
    assert_eq!(faction_squad_ids(&doc, "faction-OPFOR").len(), 3);
    assert_eq!(side_slot_count(&doc, "OPFOR"), 6);

    let s = slots(&doc);
    let root = small(&doc);
    assert_eq!(root["squadsById"]["squad-OPFOR-2"]["name"], "Squad 2");
    assert_eq!(
        root["squadsById"]["squad-OPFOR-2"]["leaderSlotId"],
        "slot-OPFOR-2-0"
    );
    assert_eq!(s["slot-OPFOR-2-1"]["callsign"], "A2-1");
    assert_eq!(s["slot-OPFOR-2-1"]["rank"], "Corporal");
    assert_eq!(s["slot-OPFOR-2-1"]["position"]["x"], 1200.0);
}

#[test]
fn apply_onto_a_single_squad_side_still_applies() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let seeded = seed_squads(&doc, "BLUFOR", 1, 3);
    let r = apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
    assert_eq!(r.squad_id, seeded[0]);
    assert_eq!(r.roles_applied, 2);
    assert_eq!(side_slot_count(&doc, "BLUFOR"), 2);
    assert_eq!(faction_squad_ids(&doc, "faction-BLUFOR").len(), 1);
}

#[test]
fn stale_squad_id_neither_refuses_nor_captures_the_apply() {
    let doc = MissionDocCore::new();
    doc.hydrate(
        &json!({
            "editor": {
                "factions": [{
                    "id": "faction-INDFOR",
                    "key": "INDFOR",
                    "name": "INDFOR",

                    "squadIds": ["squad-ghost", "squad-real"],
                }],
                "squads": [{
                    "id": "squad-real",
                    "factionId": "faction-INDFOR",
                    "name": "Real",
                    "slotIds": [],
                    "vehicleIds": [],
                }],
                "editorLayers": [{
                    "id": "lyr",
                    "name": "Layer 1",
                    "parentId": null,
                    "entityIds": [],
                }],
            }
        })
        .to_string(),
        "lyr",
    );
    assert_eq!(
        faction_squad_ids(&doc, "faction-INDFOR"),
        vec!["squad-real"]
    );

    let r = apply_faction_library(&doc, "INDFOR", "lyr", &two_role_lib()).expect("apply");
    assert_eq!(r.squad_id, "squad-real");
    assert_eq!(side_slot_count(&doc, "INDFOR"), 2);
}

#[test]
fn apply_anchors_on_the_docs_own_terrain() {
    let doc = MissionDocCore::new();
    layer(&doc);
    doc.apply_row_meta("", "arland", None, None, None);
    apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect("apply");

    let s = slots(&doc);
    assert_eq!(s["slot-OPFOR-apply-0"]["position"]["x"], 2048.0);
    assert_eq!(s["slot-OPFOR-apply-0"]["position"]["y"], 2048.0);

    assert_eq!(s["slot-OPFOR-apply-1"]["position"]["x"], 2063.0);

    let root = small(&doc);
    let v = &root["vehiclesById"]["veh-OPFOR-apply-0"]["position"];
    assert_eq!(v["x"], 2078.0);
    assert_eq!(v["y"], 2018.0);
}

#[test]
fn everon_and_unknown_terrain_keep_the_historical_anchor() {
    for terrain in ["", "everon", "custom"] {
        let doc = MissionDocCore::new();
        layer(&doc);
        if !terrain.is_empty() {
            doc.apply_row_meta("", terrain, None, None, None);
        }
        apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
        let s = slots(&doc);
        assert_eq!(
            s["slot-BLUFOR-apply-0"]["position"]["x"], 6400.0,
            "terrain {terrain:?}"
        );
        assert_eq!(
            s["slot-BLUFOR-apply-1"]["position"]["x"], 6415.0,
            "terrain {terrain:?}"
        );
        assert_eq!(
            s["slot-BLUFOR-apply-0"]["position"]["y"], 6400.0,
            "terrain {terrain:?}"
        );
    }
}

#[test]
fn two_placements_then_apply_folds_them_in() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let a = place(&doc, "BLUFOR", 0, 1111.0, 2222.0);

    doc.rename_squad(&side_squad_ids(&doc, "BLUFOR")[0], "Alpha");
    let b = place(&doc, "BLUFOR", 1, 3333.0, 4444.0);
    doc.update_slot_identity(&b, Some("A-2".into()), Some("Corporal".into()));
    assert_eq!(side_squad_ids(&doc, "BLUFOR").len(), 2, "two squads");

    let r = apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
    assert_eq!(r.roles_applied, 2);
    assert_eq!(
        r.leader_slot_id, a,
        "SL role landed on the first placed body"
    );

    let squads = side_squad_ids(&doc, "BLUFOR");
    assert_eq!(squads, vec![r.squad_id.clone()]);
    assert_eq!(
        small(&doc)["squadsById"][&r.squad_id]["name"],
        "Soviet Army 1980s"
    );
    assert_eq!(side_slot_count(&doc, "BLUFOR"), 2);

    let s = slots(&doc);
    assert_eq!(s[&a]["position"]["x"], 1111.0);
    assert_eq!(s[&a]["position"]["y"], 2222.0);
    assert_eq!(s[&b]["position"]["x"], 3333.0);
    assert_eq!(s[&b]["position"]["y"], 4444.0);
    assert_eq!(s[&a]["role"], "Squad Leader");
    assert_eq!(s[&b]["role"], "Rifleman");
    assert_eq!(s[&a]["assetId"], "{AAAA}Char.et");
    assert_eq!(s[&b]["assetId"], "{BBBB}Rifleman.et");

    assert_eq!(s[&b]["callsign"], "A-2");
    assert_eq!(s[&b]["rank"], "Corporal");
}

#[test]
fn many_placements_converge_and_reapply_is_idempotent() {
    let doc = MissionDocCore::new();
    layer(&doc);
    for i in 0..5 {
        place(&doc, "OPFOR", i, 1000.0 + 10.0 * i as f64, 2000.0);
    }

    assert_eq!(side_squad_ids(&doc, "OPFOR").len(), 1);

    apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect("first");
    assert_eq!(side_squad_ids(&doc, "OPFOR").len(), 1);
    assert_eq!(side_slot_count(&doc, "OPFOR"), 2);

    apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect("second");
    assert_eq!(side_squad_ids(&doc, "OPFOR").len(), 1);
    assert_eq!(side_slot_count(&doc, "OPFOR"), 2);

    let s = slots(&doc);
    assert_eq!(s["slot-placed-OPFOR-0"]["position"]["x"], 1000.0);
    assert_eq!(s["slot-placed-OPFOR-1"]["position"]["x"], 1010.0);
}

#[test]
fn a_legacy_per_click_document_still_folds() {
    fn legacy_place(doc: &MissionDocCore, side: &str, n: u32, x: f64, y: f64) -> String {
        let faction_id = format!("faction-{side}");
        if small(doc)["factionsById"].get(&faction_id).is_none() {
            doc.add_faction(&faction_id, side, side);
        }
        let squad_id = format!("squad-{side}-{n}");
        let ordinal = faction_squad_ids(doc, &faction_id).len();
        doc.add_squad(
            &squad_id,
            &faction_id,
            &format!("Squad {}", ordinal + 1),
            None,
        );
        let slot_id = format!("slot-legacy-{n}");
        doc.add_slot(
            &slot_id, &squad_id, "lyr", 0, "Rifleman", None, None, x, y, 0.0, 0.0,
        );
        doc.set_leader(&squad_id, &slot_id);
        slot_id
    }

    let doc = MissionDocCore::new();
    layer(&doc);
    let a = legacy_place(&doc, "OPFOR", 1, 1000.0, 2000.0);
    let b = legacy_place(&doc, "OPFOR", 2, 1010.0, 2000.0);
    legacy_place(&doc, "OPFOR", 3, 1020.0, 2000.0);
    doc.update_slot_identity(&b, Some("A-2".into()), Some("Corporal".into()));
    assert_eq!(side_squad_ids(&doc, "OPFOR").len(), 3, "pre-T-321 shape");

    let r = apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect("apply");
    assert_eq!(
        side_squad_ids(&doc, "OPFOR"),
        vec![r.squad_id.clone()],
        "folded to one"
    );
    assert_eq!(r.leader_slot_id, a);
    assert_eq!(side_slot_count(&doc, "OPFOR"), 2);

    let s = slots(&doc);
    assert_eq!(
        s[&a]["position"]["x"], 1000.0,
        "id + position survived the fold"
    );
    assert_eq!(s[&b]["position"]["x"], 1010.0);
    assert_eq!(s[&b]["callsign"], "A-2", "identity survived the fold");
    assert_eq!(s[&b]["rank"], "Corporal");
    assert_eq!(s[&a]["role"], "Squad Leader", "and took a library role");
}

#[test]
fn a_renamed_placement_squad_still_refuses() {
    let doc = MissionDocCore::new();
    layer(&doc);
    place(&doc, "BLUFOR", 0, 1111.0, 2222.0);
    doc.rename_squad(&side_squad_ids(&doc, "BLUFOR")[0], "Alpha");
    place(&doc, "BLUFOR", 1, 3333.0, 4444.0);
    let second = side_squad_ids(&doc, "BLUFOR")[1].clone();
    doc.rename_squad(&second, "Bravo");

    let small_before = small(&doc);
    let slots_before = slots(&doc);
    let err = apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect_err("refuse");
    match &err {
        ApplyFactionError::WouldCollapseSquads { blocking, .. } => {
            assert_eq!(blocking.len(), 1);
            assert_eq!(blocking[0].id, second);
            assert_eq!(blocking[0].name, "Bravo");
            assert_eq!(blocking[0].why, "renamed");
        }
        other => panic!("expected WouldCollapseSquads, got {other:?}"),
    }
    assert!(err.to_string().contains("\"Bravo\" (renamed)"), "{err}");
    assert_eq!(small(&doc), small_before, "refusal writes nothing");
    assert_eq!(slots(&doc), slots_before, "refusal writes nothing");
}

#[test]
fn a_placement_squad_with_a_vehicle_still_refuses() {
    let doc = MissionDocCore::new();
    layer(&doc);
    place(&doc, "INDFOR", 0, 1111.0, 2222.0);
    doc.rename_squad(&side_squad_ids(&doc, "INDFOR")[0], "Alpha");
    place(&doc, "INDFOR", 1, 3333.0, 4444.0);
    let second = side_squad_ids(&doc, "INDFOR")[1].clone();
    doc.add_vehicle("veh-hand", "{V}Truck.et", Some(1.0), Some(2.0), None, None);
    doc.attach_vehicle(&second, "veh-hand");

    let err = apply_faction_library(&doc, "INDFOR", "lyr", &two_role_lib()).expect_err("refuse");
    match &err {
        ApplyFactionError::WouldCollapseSquads { blocking, .. } => {
            assert_eq!(blocking.len(), 1);
            assert_eq!(blocking[0].why, "1 vehicle");
        }
        other => panic!("expected WouldCollapseSquads, got {other:?}"),
    }

    assert!(small(&doc)["vehiclesById"].get("veh-hand").is_some());
}

#[test]
fn an_empty_minted_squad_folds_away_but_a_renamed_one_blocks() {
    let doc = MissionDocCore::new();
    layer(&doc);
    place(&doc, "BLUFOR", 0, 1111.0, 2222.0);
    doc.add_squad("squad-BLUFOR-9", "faction-BLUFOR", "Squad 2", None);
    assert_eq!(side_squad_ids(&doc, "BLUFOR").len(), 2);

    apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
    assert_eq!(side_squad_ids(&doc, "BLUFOR").len(), 1, "husk dropped");
    assert!(small(&doc)["squadsById"].get("squad-BLUFOR-9").is_none());

    let doc = MissionDocCore::new();
    layer(&doc);
    place(&doc, "BLUFOR", 0, 1111.0, 2222.0);
    doc.add_squad("squad-BLUFOR-9", "faction-BLUFOR", "Weapons Det", None);
    let err = apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect_err("refuse");
    assert!(
        err.to_string().contains("\"Weapons Det\" (renamed)"),
        "{err}"
    );
}

#[test]
fn the_target_squads_own_authoring_never_blocks() {
    let doc = MissionDocCore::new();
    layer(&doc);
    let seeded = seed_squads(&doc, "BLUFOR", 1, 4);
    doc.rename_squad(&seeded[0], "Alpha");
    let r = apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
    assert_eq!(r.squad_id, seeded[0]);
    assert_eq!(side_slot_count(&doc, "BLUFOR"), 2);
}

#[test]
fn minted_squad_names_are_recognised() {
    for minted in ["Squad 1", "Squad 2", "Squad 17", "Squad 0", "", "  "] {
        assert!(is_minted_squad_name(minted), "{minted:?}");
    }
    for authored in [
        "Alpha",
        "Squad",
        "Squad A",
        "Squad ",
        "1st Squad",
        "Squad 1a",
    ] {
        assert!(!is_minted_squad_name(authored), "{authored:?}");
    }

    let doc = MissionDocCore::new();
    layer(&doc);
    place(&doc, "BLUFOR", 0, 1.0, 2.0);
    place(&doc, "BLUFOR", 1, 3.0, 4.0);
    let root = small(&doc);
    for sid in side_squad_ids(&doc, "BLUFOR") {
        let name = root["squadsById"][&sid]["name"]
            .as_str()
            .unwrap_or_default();
        assert!(is_minted_squad_name(name), "placement minted {name:?}");
    }
}

#[cfg(feature = "scenario")]
#[test]
fn apply_anchor_matches_terrain_bounds() {
    for t in ["everon", "arland", "custom", "not-a-terrain"] {
        let [min_x, min_y, max_x, max_y] = crate::data::scenario::compile::terrain_bounds(t);
        let want = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        assert_eq!(apply_anchor_for_terrain(t), want, "terrain {t:?}");
    }
}
