//! Role: Domain regression cases.
//! Position: `mission/ast/factions/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn legacy_orbat_wins() {
    let p = br#"{
            "orbat": [{"faction":"BLUFOR","callsign":"HQ","squad":"Command","slots":[
                {"role":"Commander","loadout":"","tag":""}]}],
            "editor": {"factions":[{"key":"OPFOR","squadIds":["s1"]}],
                "squads":[{"id":"s1","name":"Recon","slotIds":["x1"]}],
                "slots":[{"id":"x1","index":0,"role":"Sniper","tag":""}]}
        }"#;
    let got = parse_orbat_template(p);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].faction, "BLUFOR");
    assert_eq!(got[0].squad, "Command");
    assert_eq!(got[0].slots.len(), 1);
    assert_eq!(got[0].slots[0].role, "Commander");
}

#[test]
fn derives_from_editor_sorted_by_index() {
    let p = br#"{
            "editor": {
                "factions":[{"key":"BLUFOR","squadIds":["sq-a","sq-b"]}],
                "squads":[
                    {"id":"sq-a","callsign":"Alpha Actual","name":"Alpha 1-1","slotIds":["s2","s0","s1"]},
                    {"id":"sq-b","name":"Bravo 1-1","slotIds":["b0"]}],
                "slots":[
                    {"id":"s0","index":0,"role":"Squad Leader","tag":""},
                    {"id":"s1","index":1,"role":"Combat Medic","tag":"MED"},
                    {"id":"s2","index":2,"role":"Rifleman","tag":""},
                    {"id":"b0","index":0,"role":"Team Leader","tag":""}]}
        }"#;
    let got = parse_orbat_template(p);
    assert_eq!(got.len(), 2);
    let alpha = &got[0];
    assert_eq!(alpha.faction, "BLUFOR");
    assert_eq!(alpha.callsign, "Alpha Actual");
    assert_eq!(alpha.squad, "Alpha 1-1");
    let roles: Vec<&str> = alpha.slots.iter().map(|s| s.role.as_str()).collect();
    assert_eq!(roles, ["Squad Leader", "Combat Medic", "Rifleman"]);
    assert_eq!(alpha.slots[1].tag, "MED");
    let bravo = &got[1];
    assert_eq!(bravo.squad, "Bravo 1-1");
    assert_eq!(bravo.callsign, "");
    assert_eq!(bravo.slots.len(), 1);
    assert_eq!(bravo.slots[0].role, "Team Leader");
}

#[test]
fn derive_fills_loadout_from_summary() {
    let p = format!(
        r#"{{
            "editor": {{
                "factions":[{{"key":"BLUFOR","squadIds":["sq"]}}],
                "squads":[{{"id":"sq","name":"Alpha","slotIds":["s0"]}}],
                "slots":[{{
                    "id":"s0","index":0,"role":"Rifleman","tag":"",
                    "loadout":{{"primary":"{{AAA}}Rifle_M16A2.et","summary":"M16A2 {} ACOG"}}
                }}]
            }}
        }}"#,
        '\u{00b7}'
    );
    let got = parse_orbat_template(p.as_bytes());
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].slots.len(), 1);
    assert_eq!(got[0].slots[0].loadout, "M16A2 \u{00b7} ACOG");
}

#[test]
fn derive_fills_loadout_from_weapons() {
    let p = br#"{
            "editor": {
                "factions":[{"key":"BLUFOR","squadIds":["sq"]}],
                "squads":[{"id":"sq","name":"Alpha","slotIds":["s0"]}],
                "slots":[{
                    "id":"s0","index":0,"role":"Rifleman","tag":"",
                    "loadout":{
                        "primary":"{AAA}Rifle_M16A2.et",
                        "launcher":"PrefabLibrary/Weapons/Launchers/Launcher_M72A3.et"
                    }
                }]
            }
        }"#;
    let got = parse_orbat_template(p);
    assert_eq!(got.len(), 1);
    let lo = &got[0].slots[0].loadout;
    assert!(lo.contains("Rifle_M16A2"), "primary token: {lo}");
    assert!(lo.contains("Launcher_M72A3"), "launcher token: {lo}");
    assert!(
        lo.find("Rifle_M16A2").unwrap() < lo.find("Launcher_M72A3").unwrap(),
        "primary before launcher: {lo}"
    );
    assert!(lo.contains(" + "), "separator: {lo}");
    assert_eq!(lo, "Rifle_M16A2 + Launcher_M72A3");
}

#[test]
fn derive_empty_loadout_when_absent() {
    let p = br#"{
            "editor": {
                "factions":[{"key":"BLUFOR","squadIds":["sq"]}],
                "squads":[{"id":"sq","name":"Alpha","slotIds":["s0"]}],
                "slots":[{"id":"s0","index":0,"role":"Rifleman","tag":""}]
            }
        }"#;
    let got = parse_orbat_template(p);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].slots[0].loadout, "");
}

#[test]
fn empty_payloads_yield_nothing() {
    for p in [&b"{}"[..], b"{\"editor\":{}}", b"", b"not json"] {
        assert!(parse_orbat_template(p).is_empty(), "payload {p:?}");
    }
}

#[test]
fn skips_missing_refs() {
    let p = br#"{
            "editor": {
                "factions":[{"key":"BLUFOR","squadIds":["sq-a","ghost"]}],
                "squads":[{"id":"sq-a","name":"Alpha","slotIds":["s0","ghost-slot"]}],
                "slots":[{"id":"s0","index":0,"role":"Rifleman","tag":""}]}
        }"#;
    let got = parse_orbat_template(p);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].squad, "Alpha");
    assert_eq!(got[0].slots.len(), 1);
}

#[test]
fn faction_join_key_require_and_refuse() {
    assert_eq!(validate_faction_join_key(""), Err("faction is required"));
    assert_eq!(validate_faction_join_key("   "), Err("faction is required"));
    assert_eq!(validate_faction_join_key("\t"), Err("faction is required"));
    assert_eq!(
        validate_faction_join_key("  USA  "),
        Err("faction must not have leading or trailing whitespace")
    );
    assert_eq!(validate_faction_join_key("USA"), Ok(()));

    assert_eq!(validate_faction_join_key("US Army"), Ok(()));
}

#[test]
fn orbat_squad_faction_is_required_on_wire() {
    let missing = r#"{"callsign":"HQ","squad":"Command","slots":[]}"#;
    let err = serde_json::from_str::<OrbatSquadTemplate>(missing).unwrap_err();
    assert!(
        err.to_string().contains("faction"),
        "absent faction must fail decode, not default to empty: {err}"
    );
    let present: OrbatSquadTemplate =
        serde_json::from_str(r#"{"faction":"BLUFOR","squad":"Alpha","slots":[]}"#).unwrap();
    assert_eq!(present.faction, "BLUFOR");
}
