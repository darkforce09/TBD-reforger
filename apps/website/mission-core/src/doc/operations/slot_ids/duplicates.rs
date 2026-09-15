//! Role: duplicates.
//! Position: `doc/operations/slot_ids` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::HashMap;
use super::HashSet;
use super::MissionDocCore;

/// Duplicate slot ids using the supplied domain data.
pub fn duplicate_slot_ids(doc: &MissionDocCore) -> Vec<(String, String)> {
    let mut duplicates = Vec::new();
    let json = doc.small_maps_json();
    let parsed: serde_json::Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(_) => return duplicates,
    };

    let Some(squads) = parsed.get("squadsById").and_then(|s| s.as_object()) else {
        return duplicates;
    };

    let mut callsign_seen: HashMap<String, HashSet<String>> = HashMap::new();
    for squad in squads.values() {
        let callsign = squad
            .get("callsign")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| squad.get("name").and_then(|v| v.as_str()))
            .unwrap_or("squad");

        let Some(slot_ids) = squad.get("slotIds").and_then(|v| v.as_array()) else {
            continue;
        };

        let seen = callsign_seen.entry(callsign.to_string()).or_default();
        for id_val in slot_ids {
            if let Some(id_str) = id_val.as_str() {
                if doc.slot_exists(id_str) {
                    if !seen.insert(id_str.to_string()) {
                        duplicates.push((callsign.to_string(), id_str.to_string()));
                    }
                }
            }
        }
    }

    duplicates
}
