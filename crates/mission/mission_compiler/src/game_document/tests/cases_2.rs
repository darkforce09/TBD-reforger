//! **Role:** Vehicles, crews, win conditions and the compiled slot's exact keys.
//! **Position:** `mission_compiler::game_document::tests::cases_2` in the `mission_compiler` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

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
