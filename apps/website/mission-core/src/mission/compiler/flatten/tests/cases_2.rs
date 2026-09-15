//! Role: Domain regression cases.
//! Position: `mission/compiler/flatten/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn an_unauthored_payload_emits_the_derived_block_and_no_extension_keys() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    assert!(doc.extensions.is_empty());
    assert!(doc.win_conditions.params.is_empty());

    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(
        wire["winConditions"],
        serde_json::json!({"mode": "attrition", "endOn": ["time_limit", "faction_eliminated"]}),
        "the derived block must be EXACTLY the two keys it has always been"
    );
    for key in [
        "extractionZoneId",
        "vipSlotId",
        "timeoutMinutes",
        "tasks",
        "reserved",
    ] {
        assert!(
            wire.get(key).is_none() && wire["winConditions"].get(key).is_none(),
            "an unauthored mission must not carry `{key}`: {wire:#}"
        );
    }
    assert!(
        doc.diagnostics
            .iter()
            .all(|f| f.rule_id != DIAG_WIN_CONDITIONS),
        "a mission that authors nothing has nothing to report: {:?}",
        doc.diagnostics
    );
}

#[test]
fn each_authored_mode_reaches_the_wire_with_its_own_param() {
    let cases: [(&str, serde_json::Value, &str); 5] = [
        ("attrition", serde_json::json!({}), ""),
        ("objective", serde_json::json!({}), ""),
        (
            "extraction",
            serde_json::json!({"extractionZoneId": "z-lz"}),
            "extractionZoneId",
        ),
        ("vip", serde_json::json!({"vipSlotId": "s2"}), "vipSlotId"),
        (
            "timeout",
            serde_json::json!({"timeoutMinutes": 45}),
            "timeoutMinutes",
        ),
    ];

    for (mode, params, param_key) in cases {
        let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let mut block = serde_json::json!({"mode": mode, "endOn": ["time_limit"]});
        for (k, v) in params.as_object().expect("object") {
            block[k] = v.clone();
        }
        p["winConditions"] = block;

        let doc =
            flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("{mode} compiles");
        let wire = serde_json::to_value(&doc).expect("wire");
        let emitted = wire["winConditions"].as_object().expect("object");

        assert_eq!(emitted["mode"], mode);
        for other in ["extractionZoneId", "vipSlotId", "timeoutMinutes"] {
            assert_eq!(
                emitted.contains_key(other),
                other == param_key,
                "mode {mode} emitted `{other}`: {wire:#}"
            );
        }
    }
}

#[test]
fn win_conditions_reaches_the_wire_exactly_once() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["winConditions"] =
        serde_json::json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s1"});
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let text = serde_json::to_string(&doc).expect("serialises");
    assert_eq!(
        text.matches("\"winConditions\"").count(),
        1,
        "the typed field and the extension carrier both emitted it: {text}"
    );
}

#[test]
fn an_authored_faction_eliminated_is_dropped_on_a_one_sided_mission_and_reported() {
    let mut p: serde_json::Value =
        serde_json::from_slice(&payload_with(&[("BLUFOR", "US Army", &["Alpha"])]))
            .expect("one-sided payload parses");
    p["winConditions"] = serde_json::json!({
        "mode": "attrition", "endOn": ["faction_eliminated", "time_limit"]
    });

    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    assert_eq!(
        doc.win_conditions.end_on,
        ["time_limit"],
        "faction_eliminated must not survive onto a one-sided document"
    );
    let reported: Vec<&str> = doc
        .diagnostics
        .iter()
        .filter(|f| f.rule_id == DIAG_WIN_CONDITIONS)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(reported.len(), 1, "{:?}", doc.diagnostics);
    assert!(
        reported[0].contains("faction_eliminated"),
        "{}",
        reported[0]
    );

    let mut two: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    two["winConditions"] = serde_json::json!({
        "mode": "attrition", "endOn": ["faction_eliminated", "time_limit"]
    });
    let doc = flatten_to_mod_document(&meta(), two.to_string().as_bytes()).expect("compiles");
    assert_eq!(
        doc.win_conditions.end_on,
        ["faction_eliminated", "time_limit"]
    );
}

#[test]
fn timeout_mode_projects_onto_flow_and_adds_the_time_limit_trigger() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["winConditions"] = serde_json::json!({
        "mode": "timeout", "endOn": ["faction_eliminated"], "timeoutMinutes": 45
    });

    p["environment"] = serde_json::json!({"timeLimitSeconds": 1200});

    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    assert_eq!(doc.flow.time_limit_seconds, 45 * 60);
    assert!(
        doc.win_conditions.end_on.iter().any(|t| t == "time_limit"),
        "the clock only arms when endOn declares time_limit: {:?}",
        doc.win_conditions.end_on
    );
    assert_eq!(doc.win_conditions.params.timeout_minutes, Some(45));

    let messages: Vec<&str> = doc
        .diagnostics
        .iter()
        .filter(|f| f.rule_id == DIAG_WIN_CONDITIONS)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages.iter().any(|m| m.contains("time_limit")),
        "{messages:?}"
    );
    assert!(messages.iter().any(|m| m.contains("1200")), "{messages:?}");

    let mut agree: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    agree["winConditions"] = serde_json::json!({
        "mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": 20
    });
    agree["environment"] = serde_json::json!({"timeLimitSeconds": 1200});
    let doc = flatten_to_mod_document(&meta(), agree.to_string().as_bytes()).expect("compiles");
    assert_eq!(doc.flow.time_limit_seconds, 1200);
    assert!(
        doc.diagnostics
            .iter()
            .all(|f| f.rule_id != DIAG_WIN_CONDITIONS),
        "{:?}",
        doc.diagnostics
    );

    let mut only_rule: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    only_rule["winConditions"] = serde_json::json!({
        "mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": 45
    });
    only_rule["environment"] = serde_json::json!({});
    let doc = flatten_to_mod_document(&meta(), only_rule.to_string().as_bytes()).expect("compiles");
    assert_eq!(
        doc.flow.time_limit_seconds, 2700,
        "the win rule must still be projected onto the flow clock"
    );
    assert!(
        doc.diagnostics
            .iter()
            .all(|f| f.rule_id != DIAG_WIN_CONDITIONS),
        "an unauthored flow value cannot conflict with the win rule: {:?}",
        doc.diagnostics
    );
}

#[test]
fn a_malformed_authored_block_falls_back_to_the_derivation_and_reports() {
    for bad in [
        serde_json::json!({"mode": "vip_hunt", "endOn": ["time_limit"]}),
        serde_json::json!({"mode": "vip", "endOn": ["time_limit"]}),
        serde_json::json!({"mode": "attrition", "endOn": ["admin_ended"]}),
        serde_json::json!("attrition"),
        serde_json::json!({"mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": 0}),
    ] {
        let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        p["winConditions"] = bad.clone();
        let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes())
            .expect("a bad block must not fail the compile");

        assert_eq!(doc.win_conditions.mode, "attrition", "{bad}");
        assert_eq!(
            doc.win_conditions.end_on,
            ["time_limit", "faction_eliminated"],
            "{bad}"
        );
        assert!(doc.win_conditions.params.is_empty(), "{bad}");
        assert!(
            doc.diagnostics
                .iter()
                .any(|f| f.rule_id == DIAG_WIN_CONDITIONS),
            "a silent fallback is the defect this ticket closes: {bad}"
        );
    }
}

#[test]
fn a_dangling_reference_is_reported_and_the_value_is_carried() {
    for (mode, key, value) in [
        ("extraction", "extractionZoneId", "z-nope"),
        ("vip", "vipSlotId", "s-nope"),
    ] {
        let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        p["winConditions"] = serde_json::json!({"mode": mode, "endOn": ["time_limit"], key: value});
        let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");

        let wire = serde_json::to_value(&doc).expect("wire");
        assert_eq!(wire["winConditions"]["mode"], mode);
        assert_eq!(
            wire["winConditions"][key], value,
            "the author's value must be carried unchanged"
        );

        let finding = doc
            .diagnostics
            .iter()
            .find(|f| f.rule_id == DIAG_WIN_CONDITIONS)
            .unwrap_or_else(|| panic!("{mode}: a dangling {key} must be reported"));
        assert!(finding.message.contains(key), "{}", finding.message);
        assert_eq!(finding.subject_id.as_deref(), Some(value));
        assert_eq!(finding.subject, "/winConditions");
    }

    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["winConditions"] =
        serde_json::json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s1"});
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    assert!(
        doc.diagnostics
            .iter()
            .all(|f| f.rule_id != DIAG_WIN_CONDITIONS),
        "{:?}",
        doc.diagnostics
    );
}

#[cfg(feature = "doc")]
#[test]
fn the_vehicle_row_still_has_the_shape_this_module_reads() {
    let authored = vehicles_from_writer_json_roundtrip();
    let vehicles = authored["vehicles"]
        .as_array()
        .expect("compile_payload must emit a vehicles array from add_vehicle output");
    assert_eq!(
        vehicles.len(),
        2,
        "writer authored two vehicles (placed+attached v1, bare v2)"
    );

    let by_id = |id: &str| -> &serde_json::Value {
        vehicles
            .iter()
            .find(|v| v.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap_or_else(|| panic!("missing vehicle id={id} in writer round-trip"))
    };

    for id in ["v1", "v2"] {
        let v = by_id(id);
        for required in ["id", "resourceName"] {
            assert!(
                v.get(required)
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
                "vehicles[{id}].{required} must be a non-null string — the floor \
                     add_vehicle writes. If the writing slice renamed or retyped it, these two \
                     halves now disagree and nothing else will say so."
            );
        }
    }

    let placed = by_id("v1");
    let pos = placed["position"]
        .as_object()
        .expect("vehicles[v1].position is an object (store.rs `position_any`)");
    let mut axes: Vec<&str> = pos.keys().map(String::as_str).collect();
    axes.sort_unstable();
    assert_eq!(
        axes,
        ["rotation", "x", "y", "z"],
        "the vehicle position shape changed; `position_any` writes exactly these four"
    );
    assert!(
        pos.values().all(serde_json::Value::is_number),
        "every vehicle position axis is a number"
    );
    assert_eq!(placed["squadId"], "sq1");

    let bare = by_id("v2");
    assert!(
        bare.get("position").is_none() && bare.get("squadId").is_none(),
        "an unplaced, unattached vehicle carries neither key — a reader that requires \
             either would silently drop it"
    );

    let cargo = placed
        .get("cargo")
        .and_then(serde_json::Value::as_array)
        .expect("set_vehicle_cargo + compile_payload must preserve vehicles[].cargo");
    assert_eq!(cargo.len(), 1, "one cargo row authored");
    assert_eq!(cargo[0]["item"], "res://ammo");
    assert_eq!(cargo[0]["qty"], 4);
    assert!(
        placed
            .get("resourceName")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "floor resourceName must survive beside the cargo extra"
    );
    assert!(
        placed.get("position").and_then(|p| p.get("x")).is_some(),
        "floor position.x must survive beside the cargo extra"
    );
}

#[test]
fn a_compiled_slot_carries_exactly_these_keys() {
    let doc = flatten_to_mod_document(&meta(), LEDGER_FIXTURE.as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("serialises");

    let mut keys: Vec<&str> = wire["slots"][0]
        .as_object()
        .expect("a slot is an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "callsign",
            "faction",
            "groupCallsign",
            "headingDeg",
            "id",
            "kit",
            "rank",
            "role",
            "stance",
            "tag",
            "uid",
            "unitName",
            "x",
            "z",
        ],
        "the compiled slot shape changed. `y` and `loadout` are conditional and this \
             fixture authors neither; everything else here is unconditional. The five identity \
             keys arrived with T-674 — before it every one of them was silently discarded and no \
             test noticed, which is the loss this pin exists to make impossible a second time."
    );

    let mut top: Vec<&str> = wire
        .as_object()
        .expect("the document is an object")
        .keys()
        .map(String::as_str)
        .collect();
    top.sort_unstable();
    assert_eq!(
        top,
        [
            "entities",
            "environment",
            "factions",
            "flow",
            "meta",
            "orbat",
            "radioPlan",
            "schemaVersion",
            "slots",
            "vehicles",
            "winConditions",
            "zones",
        ],
        "the compiled document's top-level shape changed. This fixture places one vehicle, so \
             it carries BOTH of the two rows an authored vehicle produces: the T-425 `entities[]` \
             alias row and the T-675 `vehicles[]` roster row that can carry a crew plan. A mission \
             with no roster omits `vehicles` entirely."
    );
}

#[test]
fn slot_loadout_mapper_edge_cases() {
    let lo = serde_json::json!({"wear": {"vest": "res://rig", "armoredVest": ""}});
    let m = mod_slot_loadout(&lo).expect("gear");
    assert_eq!(m.gear.unwrap().vest.as_deref(), Some("res://rig"));

    let lo = serde_json::json!({
        "wear": {"jacket": ""},
        "weapons": [{"slotIndex": 1, "slotType": "primary", "weapon": "res://rpg"}],
        "cargo": [{"container": "vest", "item": "res://mag", "qty": 0}]
    });
    let m = mod_slot_loadout(&lo).expect("launcher-only loadout must survive");
    let g = m.gear.expect("launcher-only gear");
    assert_eq!(g.launcher.as_deref(), Some("res://rpg"));

    assert!(g.primary.is_none() && g.handgun.is_none() && g.throwable.is_none());
    assert!(m.cargo.is_empty());

    let lo = serde_json::json!({
        "weapons": [
            {"slotIndex": 0, "slotType": "primary",   "weapon": "res://m4", "optic": "res://acog", "magazine": "res://stanag"},
            {"slotIndex": 1, "slotType": "primary",   "weapon": "res://rpg"},
            {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9"},
            {"slotIndex": 3, "slotType": "grenade",   "weapon": "res://m67"}
        ]
    });
    let g = mod_slot_loadout(&lo)
        .expect("four weapons")
        .gear
        .expect("gear");
    assert_eq!(
        (
            g.primary.as_deref(),
            g.launcher.as_deref(),
            g.handgun.as_deref(),
            g.throwable.as_deref(),
            g.optic.as_deref(),
            g.magazine.as_deref()
        ),
        (
            Some("res://m4"),
            Some("res://rpg"),
            Some("res://m9"),
            Some("res://m67"),
            Some("res://acog"),
            Some("res://stanag")
        )
    );
    assert!(g.attachments.is_empty());

    let lo = serde_json::json!({
        "weapons": [{"slotIndex": 2, "slotType": "primary", "weapon": "res://bogus"}]
    });
    assert!(mod_slot_loadout(&lo).is_none());

    let lo = serde_json::json!({
        "weapons": [{"slotIndex": 3, "slotType": "grenade", "weapon": ""}]
    });
    assert!(mod_slot_loadout(&lo).is_none());

    let lo = serde_json::json!({"cargo": [{"container": "pants", "item": "res://b", "qty": 1}]});
    let m = mod_slot_loadout(&lo).expect("cargo-only");
    assert!(m.gear.is_none());
    assert_eq!(m.cargo.len(), 1);
}

#[test]
fn arsenal_suppressor_edge_reaches_compiled_gear() {
    const SUPPRESSOR: &str =
        "{E52C9791E1554A5F}Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/Suppressor_M16.et";
    let lo = serde_json::json!({
        "weapons": [{
            "slotIndex": 0,
            "slotType": "primary",
            "weapon": "res://m16",
            "optic": "res://acog",
            "magazine": "res://stanag",
            "attachments": [SUPPRESSOR, ""]
        }]
    });
    let g = mod_slot_loadout(&lo).expect("gear").gear.expect("gear");
    assert_eq!(g.attachments, vec![SUPPRESSOR.to_string()]);
    let wire = serde_json::to_value(&g).expect("serialize gear");
    assert_eq!(wire["attachments"][0], SUPPRESSOR);

    let lo_empty = serde_json::json!({
        "weapons": [{
            "slotIndex": 0,
            "slotType": "primary",
            "weapon": "res://m16",
            "attachments": []
        }]
    });
    let g_empty = mod_slot_loadout(&lo_empty)
        .expect("gear")
        .gear
        .expect("gear");
    let wire_empty = serde_json::to_value(&g_empty).expect("serialize empty-attach gear");
    assert!(
        wire_empty.get("attachments").is_none(),
        "empty attachments must omit the key: {wire_empty}"
    );
}

#[test]
fn empty_editor_is_no_slots() {
    let payload = br#"{"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
    assert!(matches!(
        flatten_to_mod_document(&meta(), payload),
        Err(CompileError::NoSlots)
    ));
}

#[test]
fn single_faction_compile_does_not_pad_a_phantom_opfor() {
    let payload = br#"{
          "editor": {
            "factions": [
              {"key": "BLUFOR", "name": "US", "squadIds": ["sq_a"]}
            ],
            "squads": [
              {"id": "sq_a", "callsign": "Alpha", "slotIds": ["s_a"]}
            ],
            "slots": [
              {"id": "s_a", "index": 0, "role": "RFL",
               "position": {"x": 1000.0, "y": 2000.0, "z": 0, "rotation": 0}}
            ]
          }
        }"#;
    let doc = flatten_to_mod_document(&meta(), payload).expect("compiles");
    let keys: Vec<&str> = doc.factions.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        ["blufor"],
        "phantom side padded into factions[]: {keys:?}"
    );
    assert_eq!(doc.orbat.len(), 1);
    assert!(doc.orbat.contains_key("blufor"));
    assert!(
        !doc.orbat.contains_key("opfor"),
        "phantom opfor reached ORBAT"
    );
    assert!(
        !doc.briefings.contains_key("opfor"),
        "phantom opfor reached briefings"
    );
    let wire = serde_json::to_value(&doc).unwrap();
    assert_eq!(wire["factions"].as_array().map(Vec::len), Some(1));
    assert!(
        !doc.win_conditions
            .end_on
            .iter()
            .any(|t| t == "faction_eliminated"),
        "one holding side must not declare faction_eliminated: {:?}",
        doc.win_conditions.end_on
    );
}

#[test]
fn compiled_zones_include_spawn_circles_and_terrain_boundary_fallback() {
    let doc = flatten_to_mod_document(&meta(), &zones_test_payload("[]")).expect("compiles");
    let kinds: Vec<&str> = doc.zones.iter().map(|z| z.kind.as_str()).collect();
    assert_eq!(kinds, ["spawn", "spawn", "boundary"], "{kinds:?}");
    assert!(
        doc.zones
            .iter()
            .any(|z| z.id == "z_spawn_blufor" && z.kind == "spawn"),
        "spawn disk for blufor"
    );
    let bounds = doc
        .zones
        .iter()
        .find(|z| z.id == "z_bounds")
        .expect("terrain boundary fallback");
    assert_eq!(bounds.kind, "boundary");

    let ModZoneShape::Polygon { polygon } = &bounds.shape else {
        panic!("fallback AO is a polygon, not another spawn disk");
    };
    assert_eq!(
        polygon,
        &vec![
            [0.0, 0.0],
            [12800.0, 0.0],
            [12800.0, 12800.0],
            [0.0, 12800.0],
        ]
    );
}

#[test]
fn terrain_boundary_fallback_uses_arland_4096_ring() {
    let mut arland = meta();
    arland.terrain = "arland".into();
    let doc = flatten_to_mod_document(&arland, &zones_test_payload("[]")).expect("compiles");
    let bounds = doc
        .zones
        .iter()
        .find(|z| z.id == "z_bounds")
        .expect("terrain boundary fallback");
    assert_eq!(bounds.kind, "boundary");
    let ModZoneShape::Polygon { polygon } = &bounds.shape else {
        panic!("arland fallback AO must be a polygon");
    };
    assert_eq!(
        polygon,
        &vec![[0.0, 0.0], [4096.0, 0.0], [4096.0, 4096.0], [0.0, 4096.0],]
    );
}

#[test]
fn authored_boundary_polygon_and_base_protection_circle_reach_the_compiled_document() {
    let payload = zones_test_payload(
        r#"[
              {
                "id": "z_ao",
                "type": "boundary",
                "shape": {
                  "polygon": [
                    [4500, 6400],
                    [6400, 6400],
                    [6400, 7600],
                    [4500, 7600]
                  ]
                },
                "rules": { "graceSeconds": 20, "penalty": "kill" }
              },
              {
                "id": "z_base",
                "type": "base_protection",
                "faction": "opfor",
                "shape": { "circle": { "x": 6010, "z": 7211.5, "r": 250 } }
              }
            ]"#,
    );
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    let boundary = doc
        .zones
        .iter()
        .find(|z| z.id == "z_ao")
        .expect("authored boundary");
    assert_eq!(boundary.kind, "boundary");
    let ModZoneShape::Polygon { polygon } = &boundary.shape else {
        panic!("boundary must be a polygon");
    };
    assert_eq!(polygon.len(), 4);
    assert_eq!(
        boundary.rules.as_ref().and_then(|r| r.get("penalty")),
        Some(&serde_json::json!("kill"))
    );

    let base = doc
        .zones
        .iter()
        .find(|z| z.id == "z_base")
        .expect("authored base_protection");
    assert_eq!(base.kind, "base_protection");
    assert_eq!(base.faction, "opfor");
    let ModZoneShape::Circle { circle } = &base.shape else {
        panic!("base_protection must be a circle");
    };
    assert_eq!(circle.r, 250.0);

    assert!(
        !doc.zones.iter().any(|z| z.id == "z_bounds"),
        "authored boundary supersedes the terrain fallback"
    );
    assert!(
        doc.zones.iter().filter(|z| z.kind == "spawn").count() == 2,
        "spawn synthesis still runs alongside authored play-area zones"
    );
}

#[test]
fn spawn_only_without_terrain_boundary_is_a_regression() {
    let doc = flatten_to_mod_document(&meta(), &zones_test_payload("[]")).expect("compiles");
    assert!(
        doc.zones.iter().any(|z| z.kind == "boundary"),
        "spawn-only output leaves play-area enforcement dark in TBD_ZoneRegistry"
    );
}

#[test]
fn radio_plan_is_derived_from_the_orbat() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    let plan = doc.radio_plan.as_ref().expect("radioPlan emitted");

    let seen: Vec<String> = plan
        .nets
        .iter()
        .map(|n| {
            format!(
                "{} | {} | {:.1} | {} | {}",
                n.id,
                n.label,
                n.freq_mhz,
                n.faction,
                n.range.as_deref().unwrap_or("-")
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            "net:blufor_cmd | US Army Command | 30.0 | blufor | long",
            "net:opfor_cmd | Soviet VDV Command | 30.5 | opfor | long",
            "net:blufor_alpha | Alpha | 31.0 | blufor | -",
            "net:opfor_grom | Grom | 31.5 | opfor | -",
        ]
    );

    let wire = serde_json::to_value(&doc).unwrap();
    let nets = &wire["radioPlan"]["nets"];

    assert_eq!(nets[0]["freqMHz"], 30.0);
    assert!(nets[0].get("freqMhz").is_none());

    assert_eq!(nets[0]["range"], "long");
    assert!(nets[2].get("range").is_none() && nets[3].get("range").is_none());

    let again = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    assert_eq!(serde_json::to_value(&again).unwrap(), wire);
}

#[test]
fn radio_plan_never_silences_a_side_at_the_net_cap() {
    let big: Vec<String> = (0..40).map(|i| format!("Sq{i}")).collect();
    let refs: Vec<&str> = big.iter().map(String::as_str).collect();
    let payload = payload_with(&[("blufor", "US Army", &refs), ("opfor", "Soviet VDV", &refs)]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    let nets = &doc.radio_plan.as_ref().expect("radioPlan emitted").nets;

    assert_eq!(nets.len(), MOD_MAX_NETS);

    assert_eq!(
        (nets[0].id.as_str(), nets[1].id.as_str()),
        ("net:blufor_cmd", "net:opfor_cmd")
    );

    let blufor = nets.iter().filter(|n| n.faction == "blufor").count();
    assert_eq!((blufor, nets.len() - blufor), (16, 16));
}

#[test]
fn radio_plan_ids_and_frequencies_are_unique() {
    let blufor: &[&str] = &["Alpha 1", "Alpha-1", "Alpha 1 2", "cmd"];
    let payload = payload_with(&[
        ("blufor", "US Army", blufor),
        ("opfor", "Soviet VDV", &["Alpha 1"]),
    ]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    let nets = &doc.radio_plan.as_ref().expect("radioPlan emitted").nets;

    let ids: HashSet<&str> = nets.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids.len(), nets.len(), "duplicate net id in {nets:?}");

    assert!(ids.contains("net:blufor_cmd") && ids.contains("net:blufor_cmd_2"));

    let freqs: HashSet<u64> = nets.iter().map(|n| (n.freq_mhz * 1000.0) as u64).collect();
    assert_eq!(freqs.len(), nets.len(), "duplicate frequency in {nets:?}");

    assert!(nets.iter().all(|n| (30.0..=512.0).contains(&n.freq_mhz)));

    let declared: HashSet<&str> = doc.factions.iter().map(|f| f.key.as_str()).collect();
    assert!(nets.iter().all(|n| declared.contains(n.faction.as_str())));
}
