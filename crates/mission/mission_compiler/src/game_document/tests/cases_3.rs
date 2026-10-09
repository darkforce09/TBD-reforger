//! **Role:** Zones, radio, briefings, the authored blocks and the compiler-shaped golden.
//! **Position:** `mission_compiler::game_document::tests::cases_3` in the `mission_compiler` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn an_authored_radio_plan_reaches_the_wire_unchanged() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["radioPlan"] = serde_json::json!({
        "nets": [{
            "id": "net:blufor_tac",
            "label": "Tactical",
            "freqMHz": 41.0,
            "faction": "blufor",
            "range": "long"
        }]
    });
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    let nets = &wire["radioPlan"]["nets"];
    assert_eq!(
        nets.as_array().map(Vec::len),
        Some(1),
        "authored plan is one net, not the derived ORBAT plan: {nets}"
    );
    assert_eq!(nets[0]["id"], "net:blufor_tac");
    assert_eq!(nets[0]["label"], "Tactical");
    assert_eq!(
        nets[0]["freqMHz"], 41.0,
        "the authored frequency must survive, not NET_FREQ_BASE_MHZ: {nets}"
    );
    assert_eq!(nets[0]["faction"], "blufor");
    assert_eq!(nets[0]["range"], "long");
}

#[test]
fn every_authored_block_key_reaches_the_wire() {
    for block in mission_model::authored_blocks::AUTHORED_BLOCKS {
        let sentinel = serde_json::json!({"__t946_53": block.key});
        let payload = serde_json::json!({ block.key: sentinel.clone() });
        let parsed: EditorPayload =
            serde_json::from_value(payload).expect("EditorPayload accepts any shape");
        assert_eq!(
            parsed.authored_block_value(block.key),
            Some(&sentinel),
            "`{}` is registered in AUTHORED_BLOCKS but EditorPayload drops it: it needs a \
                 named #[serde(rename)] field AND an authored_block_value arm, or /compiled will \
                 silently omit every mission that authors it (T-946.53)",
            block.key
        );
        assert_eq!(
            parsed.authored_blocks_root().get(block.key),
            Some(&sentinel),
            "`{}` never reaches the extensions root",
            block.key
        );
    }
}

#[test]
fn unaliased_character_is_recorded_not_swallowed() {
    let payload = payload_with_assets(&[
        (
            "BLUFOR",
            "US Army",
            "Alpha",
            &[US_SNIPER, US_RIFLEMAN_ALIASED, US_SNIPER],
        ),
        ("OPFOR", "Soviet VDV", "Grom", &[USSR_MEDIC]),
    ]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");

    let kits: Vec<&str> = doc.slots.iter().map(|s| s.kit.as_str()).collect();
    assert_eq!(
        kits,
        [
            "kit:us_rifleman",
            "kit:us_rifleman",
            "kit:us_rifleman",
            "kit:sov_rifleman"
        ]
    );

    let rep = &doc.kit_substitutions;
    assert!(!rep.is_empty());

    assert_eq!(rep.slots(), 3);
    assert_eq!(rep.rows().len(), 2, "{:?}", rep.rows());

    let sniper = &rep.rows()[0];
    assert_eq!(sniper.asset_id, US_SNIPER);
    assert_eq!(sniper.faction, "blufor");
    assert_eq!(sniper.kit, "kit:us_rifleman");

    assert_eq!(sniper.example_slot_id, "blufor:Alpha:RFL:0");
    assert_eq!(sniper.example_slot_uid, "f0s0");
    assert_eq!(sniper.occurrences, 2);

    let medic = &rep.rows()[1];
    assert_eq!(
        (
            medic.asset_id.as_str(),
            medic.faction.as_str(),
            medic.kit.as_str(),
            medic.occurrences
        ),
        (USSR_MEDIC, "opfor", "kit:sov_rifleman", 1)
    );

    let lines = rep.details();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("blufor:Alpha:RFL:0:"), "{lines:?}");
    assert!(lines[0].contains("Character_US_Sniper.et"), "{lines:?}");
    assert!(lines[0].contains("kit:us_rifleman"), "{lines:?}");
    assert!(lines[0].contains("and 1 more slot(s)"), "{lines:?}");

    assert!(
        !lines.iter().any(|l| l.starts_with('+')),
        "nothing was dropped: {lines:?}"
    );
}

#[test]
fn substitutions_never_reach_the_compiled_wire() {
    let payload = payload_with_assets(&[
        ("BLUFOR", "US Army", "Alpha", &[US_SNIPER]),
        ("OPFOR", "Soviet VDV", "Grom", &[USSR_MEDIC]),
    ]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    assert!(!doc.kit_substitutions.is_empty(), "fixture must substitute");

    let wire = serde_json::to_value(&doc).unwrap();

    let mut keys: Vec<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "environment",
            "factions",
            "flow",
            "meta",
            "orbat",
            "radioPlan",
            "schemaVersion",
            "slots",
            "winConditions",
            "zones"
        ],
        "top-level key set is the document contract — nothing new may appear here"
    );

    let text = serde_json::to_string(&doc).unwrap();
    assert!(!text.contains("ubstitution"), "report leaked into the wire");

    assert!(!text.contains("Character_US_Sniper"), "{text}");
}

#[test]
fn a_control_character_in_an_asset_id_is_escaped_but_nothing_is_truncated() {
    assert_eq!(
        escape_resource_name(US_SNIPER),
        US_SNIPER,
        "clean is verbatim"
    );

    assert!(US_SNIPER.chars().count() > 60);

    let dirty = format!("{US_SNIPER}\t");
    let out = escape_resource_name(&dirty);
    assert!(!out.contains('\t'), "{out}");
    assert!(out.contains("\\u{09}"), "{out}");
    assert!(
        out.contains("Character_US_Sniper.et"),
        "the name must survive: {out}"
    );
    assert!(!out.contains('…'), "nothing is elided: {out}");
}

#[test]
#[ignore = "manual golden regeneration"]
fn regen_compiler_shaped_fixture() {
    let doc = flatten_to_mod_document(&compiler_shaped_meta(), COMPILER_SHAPED_PAYLOAD.as_bytes())
        .expect("compiles");
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/fixtures/missions/valid/compiler-shaped-two-faction.json"
    );
    std::fs::write(path, golden_text(&doc)).expect("write golden");
    eprintln!("regenerated {path}");
}

#[test]
fn compiler_shaped_golden_is_a_fresh_emitter_output() {
    let doc = flatten_to_mod_document(&compiler_shaped_meta(), COMPILER_SHAPED_PAYLOAD.as_bytes())
        .expect("the compiler-shaped fixture compiles");

    assert!(
        doc.kit_substitutions.rows().is_empty(),
        "fixture must name only aliased characters: {:?}",
        doc.kit_substitutions.details()
    );

    let regenerated = golden_text(&doc);
    if let Some((line, expected, actual)) =
        first_line_difference(COMPILER_SHAPED_GOLDEN, &regenerated)
    {
        panic!(
            "contracts/fixtures/missions/valid/compiler-shaped-two-faction.json is no \
                 longer this emitter's output.\n\
                 First difference at line {line}:\n  \
                 committed:   {expected}\n  \
                 regenerated: {actual}\n\n\
                 If the emitter change was intentional, REGENERATE the file — do not hand-patch \
                 the delta in, or it stops being a byte-faithful compiler output and this gate \
                 stops meaning anything:\n  \
                 let mut s = serde_json::to_string_pretty(&doc).unwrap();\n  \
                 s.push('\\n');\n\n\
                 Then re-run `cargo xtask mod world-boot \
                 --mission=compiler-shaped-two-faction`, because a regenerated document is a \
                 new document as far as the mod's parser is concerned."
        );
    }
}

#[test]
fn authored_briefings_reproduce_the_committed_golden_block() {
    let golden: serde_json::Value = serde_json::from_str(BRIDGEHEAD_GOLDEN).expect("golden parses");
    let expected = golden
        .get("briefings")
        .expect("bridgehead-at-levie.json carries a briefings block")
        .clone();

    assert!(expected.get("blufor").is_some() && expected.get("opfor").is_some());

    let actual = compiled_briefings(&payload_with_briefings(expected.clone()));
    assert_eq!(
        numbers_as_f64(&actual),
        numbers_as_f64(&expected),
        "the emitter must pass an authored briefings block through unchanged"
    );

    assert_eq!(
        actual["blufor"]["markers"][0]["x"],
        serde_json::json!(5402.0)
    );
    assert_eq!(
        expected["blufor"]["markers"][0]["x"],
        serde_json::json!(5402)
    );
    assert_eq!(
        actual["blufor"]["markers"][0]["label"], expected["blufor"]["markers"][0]["label"],
        "every non-numeric field must be EXACTLY equal, no normalisation"
    );

    let doc =
        flatten_to_mod_document(&meta(), &payload_with_briefings(expected)).expect("compiles");
    let bytes = serde_json::to_string_pretty(&doc).expect("serialises");
    let block = &bytes[bytes.find("\"briefings\"").expect("briefings in the bytes")..];
    for keys in [
        ["situation", "mission", "execution", "markers"],
        ["x", "z", "icon", "label"],
    ] {
        let mut last = 0;
        for key in keys {
            let at = block
                .find(&format!("\"{key}\""))
                .unwrap_or_else(|| panic!("{key} present in {block}"));
            assert!(at > last, "{key} out of schema order in {block}");
            last = at;
        }
    }
}

#[test]
fn briefing_keys_are_slugged_onto_the_faction_keys_the_mod_looks_up() {
    let doc = flatten_to_mod_document(
        &meta(),
        &payload_with_briefings(serde_json::json!({
            "BLUFOR": {"situation": "west bank"},
            "OPFOR":  {"situation": "east bank"},
        })),
    )
    .expect("compiles");

    let keys: Vec<&str> = doc.briefings.keys().map(String::as_str).collect();
    assert_eq!(keys, ["blufor", "opfor"]);

    let faction_keys: Vec<&str> = doc.factions.iter().map(|f| f.key.as_str()).collect();
    for k in doc.briefings.keys() {
        assert!(
            faction_keys.contains(&k.as_str()),
            "briefings key {k:?} matches no compiled faction {faction_keys:?} — \
                 GetBriefingForFaction would miss and the side would get no orders"
        );
        assert!(
            doc.orbat.contains_key(k),
            "briefings key {k:?} is not an orbat key — the two must agree on faction identity"
        );
    }
}

#[test]
fn every_marker_key_is_emitted_even_when_empty() {
    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
        "blufor": {"markers": [{"x": 7615.0, "z": 4350.0, "icon": "", "label": ""}]},
    })));

    let marker = out["blufor"]["markers"][0]
        .as_object()
        .expect("marker object");
    let mut keys: Vec<&str> = marker.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["icon", "label", "x", "z"],
        "exactly the four required keys — no omissions (required) and no extras \
             (additionalProperties: false)"
    );
    assert_eq!(marker["icon"], "");
    assert_eq!(marker["label"], "");

    let entry = out["blufor"].as_object().expect("briefing object");
    assert_eq!(
        entry.keys().map(String::as_str).collect::<Vec<_>>(),
        ["markers"]
    );
}
