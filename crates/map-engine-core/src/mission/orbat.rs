//! ORBAT template parsing — Rust port of `services/mission_payload.go`. Derives the
//! event ORBAT (factions/squads/roles) from a mission version payload, mirroring
//! `compile.ts` traversal order exactly.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbatSquadTemplate {
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub callsign: String,
    #[serde(default)]
    pub squad: String,
    #[serde(default)]
    pub slots: Vec<OrbatSlotTemplate>,
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

/// Reconstruct the ORBAT from the editor graph, mirroring `compile.ts` order EXACTLY:
/// factions in array order → each `squadIds` → resolve squad → each `slotIds` →
/// resolve slots → sort by `index` ascending. `loadout` is always `""`.
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
                    loadout: String::new(),
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
        assert!(alpha.slots.iter().all(|s| s.loadout.is_empty()));
        assert_eq!(alpha.slots[1].tag, "MED");
        let bravo = &got[1];
        assert_eq!(bravo.squad, "Bravo 1-1");
        assert_eq!(bravo.callsign, "");
        assert_eq!(bravo.slots.len(), 1);
        assert_eq!(bravo.slots[0].role, "Team Leader");
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
}
