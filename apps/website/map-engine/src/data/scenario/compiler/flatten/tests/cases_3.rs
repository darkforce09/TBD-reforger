//! Role: Domain regression cases.
//! Position: `mission/compiler/flatten/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn radio_plan_label_is_capped_at_the_mod_limit() {
    let long_name = "N".repeat(200);
    let long_callsign = "C".repeat(200);
    let payload = payload_with(&[
        ("blufor", &long_name, &[long_callsign.as_str()]),
        ("opfor", "", &["Grom"]),
    ]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    let nets = &doc.radio_plan.as_ref().expect("radioPlan emitted").nets;

    assert!(
        nets.iter()
            .all(|n| (1..=MOD_MAX_LABEL_CHARS).contains(&n.label.chars().count()))
    );
    assert_eq!(nets[0].label, "N".repeat(MOD_MAX_LABEL_CHARS));

    assert_eq!(nets[1].label, "opfor Command");
}

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
fn authored_tasks_survive_flatten_to_mod_document() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["tasks"] = serde_json::json!([{
        "id": "t-pri",
        "title": "Seize the hill",
        "tier": "primary",
        "state": "assigned"
    }]);
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(
        wire["tasks"][0]["id"], "t-pri",
        "tasks[] must survive flatten_to_mod_document, not vanish at authored_blocks_root: {wire:#}"
    );
    assert_eq!(wire["tasks"][0]["tier"], "primary");
    assert_eq!(wire["tasks"].as_array().map(Vec::len), Some(1));
}

#[test]
fn authored_spawn_modules_survive_flatten_to_mod_document() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["spawnModules"] = serde_json::json!([
        {
            "id": "sm-wave",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
            "x": 1200.0,
            "z": 3400.0,
            "count": 2,
            "intervalSeconds": 45.0,
            "maxAlive": 4
        },
        {
            "id": "sm-gar",
            "kind": "garrison",
            "factionKey": "blufor",
            "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
            "zoneId": "z_spawn_blufor",
            "count": 1
        }
    ]);
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(
        wire["spawnModules"][0]["kind"], "wave",
        "spawnModules must survive flatten_to_mod_document: {wire:#}"
    );
    assert_eq!(wire["spawnModules"][1]["kind"], "garrison");
    assert_eq!(wire["spawnModules"].as_array().map(Vec::len), Some(2));
    assert_eq!(wire["spawnModules"][0]["factionKey"], "opfor");
    assert_eq!(wire["spawnModules"][1]["zoneId"], "z_spawn_blufor");
}

#[test]
fn authored_tactical_graphics_survive_flatten_to_mod_document() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["tacticalGraphics"] = serde_json::json!([
        {
            "id": "tg-phase",
            "kind": "phase_line",
            "points": [[1000.0, 2000.0], [1400.0, 2100.0]],
            "label": "PL BLUE",
            "sideKey": "blufor",
            "style": {"color": "#3388ff", "alpha": 0.8}
        },
        {
            "id": "tg-bound",
            "kind": "boundary",
            "points": [[900.0, 1800.0], [1200.0, 1900.0], [1500.0, 2400.0]]
        },
        {
            "id": "tg-axis",
            "kind": "axis_of_advance",
            "points": [[800.0, 1200.0], [1600.0, 2600.0]],
            "label": "AXIS SABRE"
        },
        {
            "id": "tg-arrow",
            "kind": "curved_arrow",
            "points": [[700.0, 1100.0], [1100.0, 1500.0], [1700.0, 1400.0]],
            "style": {"brush": "solid", "color": "#ff2222", "widthM": 24.0}
        }
    ]);
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(
        wire["tacticalGraphics"][0]["kind"], "phase_line",
        "tacticalGraphics must survive flatten_to_mod_document: {wire:#}"
    );
    assert_eq!(wire["tacticalGraphics"].as_array().map(Vec::len), Some(4));
    assert_eq!(wire["tacticalGraphics"][1]["kind"], "boundary");
    assert_eq!(wire["tacticalGraphics"][2]["kind"], "axis_of_advance");
    assert_eq!(wire["tacticalGraphics"][3]["kind"], "curved_arrow");
    assert_eq!(wire["tacticalGraphics"][0]["label"], "PL BLUE");
    assert_eq!(wire["tacticalGraphics"][0]["sideKey"], "blufor");
    assert_eq!(wire["tacticalGraphics"][3]["style"]["color"], "#ff2222");
}

#[test]
fn every_authored_block_key_reaches_the_wire() {
    for block in crate::data::scenario::extensions::AUTHORED_BLOCKS {
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
fn a_slot_that_named_no_character_is_not_a_substitution() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    assert!(
        doc.kit_substitutions.is_empty(),
        "{:?}",
        doc.kit_substitutions.rows()
    );
    assert!(doc.kit_substitutions.details().is_empty());

    assert_eq!(doc.slots[1].kit, "kit:us_rifleman");

    let payload = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &["", US_SNIPER])]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    assert_eq!(doc.kit_substitutions.slots(), 1, "only the sniper counts");
    assert_eq!(
        doc.kit_substitutions.rows()[0].example_slot_id,
        "blufor:Alpha:RFL:1"
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
fn the_same_character_on_two_sides_is_two_substitutions() {
    let payload = payload_with_assets(&[
        ("BLUFOR", "US Army", "Alpha", &[USSR_MEDIC]),
        ("OPFOR", "Soviet VDV", "Grom", &[USSR_MEDIC]),
    ]);
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");
    let rows = doc.kit_substitutions.rows();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(
        (rows[0].faction.as_str(), rows[0].kit.as_str()),
        ("blufor", "kit:us_rifleman")
    );
    assert_eq!(
        (rows[1].faction.as_str(), rows[1].kit.as_str()),
        ("opfor", "kit:sov_rifleman")
    );
    assert!(rows.iter().all(|r| r.asset_id == USSR_MEDIC));
}

#[test]
fn repeats_collapse_and_the_cap_keeps_the_count_honest() {
    let bulk: Vec<&str> = std::iter::repeat_n(US_SNIPER, 500).collect();
    let payload = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &bulk)]);
    let rep = flatten_to_mod_document(&meta(), &payload)
        .expect("compiles")
        .kit_substitutions;
    assert_eq!(rep.rows().len(), 1, "a bulk paste is one finding");
    assert_eq!((rep.rows()[0].occurrences, rep.slots()), (500, 500));
    assert_eq!(rep.details().len(), 1);

    let owned: Vec<String> = (0..MAX_REPORTED_SUBSTITUTIONS + 5)
        .map(|i| format!("{{DEADBEEF{i:08X}}}Prefabs/Characters/Made/Up_{i}.et"))
        .collect();
    let mut many: Vec<&str> = owned.iter().map(String::as_str).collect();
    many.push(owned[0].as_str());
    let payload = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &many)]);
    let rep = flatten_to_mod_document(&meta(), &payload)
        .expect("compiles")
        .kit_substitutions;
    assert_eq!(rep.rows().len(), MAX_REPORTED_SUBSTITUTIONS);
    assert_eq!(rep.slots(), MAX_REPORTED_SUBSTITUTIONS + 6);

    let lines = rep.details();
    assert_eq!(lines.len(), MAX_REPORTED_SUBSTITUTIONS + 1);
    assert!(
        lines[MAX_REPORTED_SUBSTITUTIONS].starts_with("+ 5 further slot(s)"),
        "{:?}",
        lines[MAX_REPORTED_SUBSTITUTIONS]
    );
}

#[test]
fn the_compile_erases_the_authored_character_and_only_the_report_can_say_so() {
    let meta_json = serde_json::to_vec(&meta()).expect("meta serializes");
    let placed_sniper = payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &[US_SNIPER])]);
    let placed_aliased =
        payload_with_assets(&[("BLUFOR", "US Army", "Alpha", &[US_RIFLEMAN_ALIASED])]);

    let (sniper_bytes, sniper_lines) =
        flatten_mod_document_json_with_substitutions(&meta_json, &placed_sniper).expect("compiles");
    let (aliased_bytes, aliased_lines) =
        flatten_mod_document_json_with_substitutions(&meta_json, &placed_aliased)
            .expect("compiles");

    assert_eq!(
        sniper_bytes, aliased_bytes,
        "the compile erases which character was placed"
    );
    let doc: serde_json::Value = serde_json::from_slice(&sniper_bytes).expect("valid json");
    assert_eq!(doc["slots"][0]["kit"], "kit:us_rifleman");
    assert_eq!(doc["slots"][0]["id"], "blufor:Alpha:RFL:0");

    assert!(
        aliased_lines.is_empty(),
        "an aliased character is not a substitution: {aliased_lines:?}"
    );
    assert_eq!(sniper_lines.len(), 1, "{sniper_lines:?}");

    assert!(
        sniper_lines[0].starts_with("blufor:Alpha:RFL:0:"),
        "{sniper_lines:?}"
    );
    assert!(
        sniper_lines[0].contains("Character_US_Sniper.et"),
        "{sniper_lines:?}"
    );
    assert!(
        sniper_lines[0].contains("kit:us_rifleman"),
        "{sniper_lines:?}"
    );

    assert_eq!(
        flatten_mod_document_json(&meta_json, &placed_sniper).expect("compiles"),
        sniper_bytes,
        "the byte-identical-to-/compiled contract must survive T-314"
    );
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
        "/../../packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json"
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
            "packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json is no \
                 longer this emitter's output.\n\
                 First difference at line {line}:\n  \
                 committed:   {expected}\n  \
                 regenerated: {actual}\n\n\
                 If the emitter change was intentional, REGENERATE the file — do not hand-patch \
                 the delta in, or it stops being a byte-faithful compiler output and this gate \
                 stops meaning anything:\n  \
                 let mut s = serde_json::to_string_pretty(&doc).unwrap();\n  \
                 s.push('\\n');\n\n\
                 Then re-run `scripts/mod/world-boot.sh \
                 --mission=compiler-shaped-two-faction`, because a regenerated document is a \
                 new document as far as the mod's parser is concerned."
        );
    }
}

#[test]
fn the_guard_catches_the_t203_radio_plan_drift() {
    let open = COMPILER_SHAPED_GOLDEN
        .find("  \"radioPlan\": {\n")
        .expect("the regenerated golden carries a radioPlan block");
    let close = COMPILER_SHAPED_GOLDEN[open..]
        .find("\n  },\n")
        .expect("radioPlan block is closed")
        + open
        + "\n  },\n".len();
    let pre_t203: String = format!(
        "{}{}",
        &COMPILER_SHAPED_GOLDEN[..open],
        &COMPILER_SHAPED_GOLDEN[close..]
    );

    assert!(!pre_t203.contains("radioPlan"));
    assert!(pre_t203.contains("\"zones\"") && pre_t203.contains("\"slots\""));
    assert!(pre_t203.lines().count() < COMPILER_SHAPED_GOLDEN.lines().count());

    let (line, expected, actual) = first_line_difference(&pre_t203, COMPILER_SHAPED_GOLDEN).expect(
        "dropping radioPlan MUST register as a difference — otherwise the guard is a \
                     no-op and the golden can rot again exactly the way it just did",
    );
    assert!(
        actual.contains("radioPlan"),
        "the diff must point AT the dropped block, not somewhere downstream of it \
             (line {line}: {expected:?} vs {actual:?})"
    );
}

#[test]
fn briefings_is_omitted_entirely_when_nothing_authors_one() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    assert!(doc.briefings.is_empty());

    let text = serde_json::to_string(&doc).expect("serialises");
    assert!(
        !text.contains("briefings"),
        "an unauthored briefings block must not reach the bytes at all: {text}"
    );

    assert_eq!(
        compiled_briefings(&payload_with_briefings(
            serde_json::json!({"BLUFOR": serde_json::Value::Null})
        )),
        serde_json::Value::Null
    );

    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({"BLUFOR": {}})));
    assert_eq!(out["blufor"], serde_json::json!({}));
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
fn prose_newlines_survive_because_briefing_prose_is_wire_exempt() {
    let prose = "Soviet airborne hold the crossing.\n\nSecond paragraph.\nThird line.";
    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
        "blufor": {"situation": prose, "mission": "Seize it.", "execution": "Advance."},
    })));

    assert_eq!(
        out["blufor"]["situation"].as_str().expect("string"),
        prose,
        "prose must reach the mod exactly as authored, newlines included"
    );
}

#[test]
fn marker_labels_are_not_sanitised_because_markers_are_not_a_delimited_wire() {
    let label = "AL\tPHA\nBRAVO";
    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
        "blufor": {"markers": [{"x": 1.0, "z": 2.0, "icon": "objective", "label": label}]},
    })));

    assert_eq!(
        out["blufor"]["markers"][0]["label"].as_str().expect("str"),
        label,
        "a marker label must pass through verbatim — see this test's doc comment for why"
    );
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

#[test]
fn marker_labels_are_capped_at_the_mods_budget() {
    let long = "M".repeat(MOD_MAX_MARKER_LABEL_CHARS + 40);
    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
        "blufor": {"markers": [{"x": 1.0, "z": 2.0, "icon": "dot", "label": long}]},
    })));
    assert_eq!(
        out["blufor"]["markers"][0]
            .get("label")
            .and_then(serde_json::Value::as_str)
            .expect("str")
            .chars()
            .count(),
        MOD_MAX_MARKER_LABEL_CHARS
    );

    let wide = "Ω".repeat(MOD_MAX_MARKER_LABEL_CHARS + 5);
    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
        "blufor": {"markers": [{"x": 1.0, "z": 2.0, "icon": "dot", "label": wide}]},
    })));
    let got = out["blufor"]["markers"][0]["label"]
        .as_str()
        .expect("str")
        .to_string();
    assert_eq!(got.chars().count(), MOD_MAX_MARKER_LABEL_CHARS);
    assert!(got.chars().all(|c| c == 'Ω'), "cut mid-sequence: {got:?}");
}
