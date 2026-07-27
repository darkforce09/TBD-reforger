//! ORBAT template parsing — Rust port of `services/mission_payload.go`. Derives the
//! event ORBAT (factions/squads/roles) from a mission version payload, mirroring
//! `compile.ts` traversal order exactly.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One ordered, distinct slot in a squad: a role + loadout + optional tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbatSlotTemplate {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub loadout: String,
    #[serde(default)]
    pub tag: String,
}

/// A squad + its ordered slot list (list position = its 1-based ORBAT number).
///
/// **`faction` is deliberately required — do not add `#[serde(default)]` (T-356).**
/// It is the Event Hub join key against `mission_armories.faction` (T-346's mirror).
/// An absent key used to decode as `""` and land in `orbat_slots.faction`, matching no
/// armory group. Empty / whitespace-only / padded values are refused at the API write
/// boundary via [`validate_faction_join_key`] — never trimmed, never silently normalised.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbatSquadTemplate {
    pub faction: String,
    #[serde(default)]
    pub callsign: String,
    #[serde(default)]
    pub squad: String,
    #[serde(default)]
    pub slots: Vec<OrbatSlotTemplate>,
}

/// T-356 / T-346 mirror: `orbat_slots.faction` is a **join key**, matched byte-for-byte
/// against `mission_armories.faction`. Require a non-empty value after trim; refuse a value
/// that differs from its trimmed form; callers must **store the original bytes verbatim**.
///
/// Do NOT trim one side only — a unilateral trim breaks the case where ORBAT `"  USA  "` and
/// armory `"  USA  "` agree today. Normalising both sides needs a migration for live rows.
pub fn validate_faction_join_key(faction: &str) -> Result<(), &'static str> {
    if faction.trim().is_empty() {
        return Err("faction is required");
    }
    if faction != faction.trim() {
        return Err("faction must not have leading or trailing whitespace");
    }
    Ok(())
}

/// Extract the ORBAT squad list: an explicit top-level `"orbat"` array wins;
/// otherwise it is derived from the editor graph (Save Version omits `orbat`).
pub fn parse_orbat_template(payload: &[u8]) -> Vec<OrbatSquadTemplate> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Top {
        orbat: Vec<OrbatSquadTemplate>,
    }
    let top: Top = serde_json::from_slice(payload).unwrap_or_default();
    if !top.orbat.is_empty() {
        return top.orbat;
    }
    derive_orbat_from_editor(payload)
}

/// Human-readable kit string for Event/Export `orbat[].slots[].loadout`.
/// Prefers embedded `summary`; else joins display names of `primary` + `launcher` with `" + "`.
fn loadout_summary_from_value(lo: Option<&Value>) -> String {
    let Some(lo) = lo.filter(|v| !v.is_null()) else {
        return String::new();
    };
    if let Some(s) = lo.get("summary").and_then(Value::as_str) {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let primary = lo
        .get("primary")
        .and_then(Value::as_str)
        .map(display_resource)
        .filter(|s| !s.is_empty());
    let launcher = lo
        .get("launcher")
        .and_then(Value::as_str)
        .map(display_resource)
        .filter(|s| !s.is_empty());
    [primary, launcher]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Basename after last `/` or `}`, then strip trailing `.et`.
fn display_resource(resource: &str) -> String {
    let base = resource
        .rsplit(['/', '}'])
        .next()
        .unwrap_or(resource)
        .trim();
    base.strip_suffix(".et").unwrap_or(base).to_string()
}

/// Reconstruct the ORBAT from the editor graph, mirroring `compile.ts` order EXACTLY:
/// factions in array order → each `squadIds` → resolve squad → each `slotIds` →
/// resolve slots → sort by `index` ascending. `loadout` is the slot summary (or
/// primary+launcher rebuild) when present; `""` when absent.
pub fn derive_orbat_from_editor(payload: &[u8]) -> Vec<OrbatSquadTemplate> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Ep {
        editor: Eg,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Eg {
        factions: Vec<F>,
        squads: Vec<S>,
        slots: Vec<Sl>,
    }
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase", default)]
    struct F {
        key: String,
        squad_ids: Vec<String>,
    }
    #[derive(Deserialize, Default)]
    #[serde(rename_all = "camelCase", default)]
    struct S {
        id: String,
        callsign: String,
        name: String,
        slot_ids: Vec<String>,
    }
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Sl {
        id: String,
        index: i64,
        role: String,
        tag: String,
        loadout: Option<Value>,
    }

    let Ok(e) = serde_json::from_slice::<Ep>(payload) else {
        return Vec::new();
    };
    let ed = e.editor;
    if ed.factions.is_empty() {
        return Vec::new();
    }

    let squads_by_id: HashMap<&str, &S> = ed.squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let slots_by_id: HashMap<&str, &Sl> = ed.slots.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut out: Vec<OrbatSquadTemplate> = Vec::new();
    for f in &ed.factions {
        for squad_id in &f.squad_ids {
            let Some(sq) = squads_by_id.get(squad_id.as_str()) else {
                continue;
            };
            let mut rows: Vec<&Sl> = sq
                .slot_ids
                .iter()
                .filter_map(|id| slots_by_id.get(id.as_str()).copied())
                .collect();
            rows.sort_by_key(|s| s.index); // stable
            let slots = rows
                .iter()
                .map(|r| OrbatSlotTemplate {
                    role: r.role.clone(),
                    loadout: loadout_summary_from_value(r.loadout.as_ref()),
                    tag: r.tag.clone(),
                })
                .collect();
            out.push(OrbatSquadTemplate {
                faction: f.key.clone(),
                callsign: sq.callsign.clone(),
                squad: sq.name.clone(),
                slots,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_orbat_wins() {
        // Both a top-level orbat and an editor block → the explicit orbat wins verbatim.
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
        // Editor-only; slot ids listed out of index order → derivation sorts ascending.
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
        // Middle-dot in summary must not live in a `br#` literal (non-ASCII ban).
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

    /// T-356 — join-key require-and-refuse (T-346 mirror). Empty / whitespace-only / padded
    /// refuse; interior whitespace and clean keys pass. Callers store verbatim.
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
        // Over-rejection pin (T-346 `"US Army"`): interior whitespace is a legitimate key.
        assert_eq!(validate_faction_join_key("US Army"), Ok(()));
    }

    /// T-356 — absent `faction` must not decode as `""` via `#[serde(default)]`.
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
}
