use map_engine_core::doc::MissionDocCore;
use std::collections::{HashMap, HashSet};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duplicate_slot_ids() {
        let doc = MissionDocCore::new();
        doc.add_squad("sq1", "f1", "Alpha", Some("1-1".to_string()));
        doc.add_slot("s1", "sq1", "l1", 0, "RFL", None, None, 0.0, 0.0, 0.0, 0.0);
        doc.add_slot("s1", "sq1", "l1", 1, "MED", None, None, 0.0, 0.0, 0.0, 0.0);

        let dups = duplicate_slot_ids(&doc);
        assert!(!dups.is_empty());
        assert_eq!(dups[0], ("1-1".to_string(), "s1".to_string()));
    }
}
