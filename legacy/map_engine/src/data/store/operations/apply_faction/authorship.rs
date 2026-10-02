//! Role: authorship.
//! Position: `doc/operations/apply_faction` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::AuthoredSquad;
use super::MissionDocCore;
use super::Value;

/// Faction squad ids using the supplied domain data.
pub(super) fn faction_squad_ids(doc: &MissionDocCore, faction_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    let live = root.get("squadsById").and_then(Value::as_object);
    root.get("factionsById")
        .and_then(|v| v.get(faction_id))
        .and_then(|f| f.get("squadIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|id| live.is_some_and(|m| m.contains_key(*id)))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Squad authorship using the supplied domain data.
pub(super) fn squad_authorship(doc: &MissionDocCore, squad_id: &str) -> Option<AuthoredSquad> {
    let root = serde_json::from_str::<Value>(&doc.small_maps_json()).ok()?;
    let sq = root.get("squadsById")?.get(squad_id)?;

    let name = sq
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let slots = sq
        .get("slotIds")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let vehicles = sq
        .get("vehicleIds")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let callsign = sq
        .get("callsign")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();

    let why = if slots > 1 {
        plural(slots, "slot")
    } else if !is_minted_squad_name(&name) {
        "renamed".to_string()
    } else if !callsign.is_empty() {
        format!("callsign {callsign}")
    } else if vehicles > 0 {
        plural(vehicles, "vehicle")
    } else {
        return None;
    };

    Some(AuthoredSquad {
        id: squad_id.to_string(),
        name: if name.trim().is_empty() {
            squad_id.to_string()
        } else {
            name
        },
        why,
        slots,
    })
}

/// Plural using the supplied domain data.
pub(super) fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// Is minted squad name using the supplied domain data.
pub(super) fn is_minted_squad_name(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || n.strip_prefix("Squad ")
            .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Squad slot ids using the supplied domain data.
pub(super) fn squad_slot_ids(doc: &MissionDocCore, squad_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    root.get("squadsById")
        .and_then(|m| m.get(squad_id))
        .and_then(|sq| sq.get("slotIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Squad vehicle ids using the supplied domain data.
pub(super) fn squad_vehicle_ids(doc: &MissionDocCore, squad_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    root.get("squadsById")
        .and_then(|m| m.get(squad_id))
        .and_then(|sq| sq.get("vehicleIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Mint slot id using the supplied domain data.
pub(super) fn mint_slot_id(doc: &MissionDocCore, side: &str, i: usize) -> String {
    let existing = existing_slot_ids(doc);
    let base = format!("slot-{side}-apply-{i}");
    if !existing.contains(&base) {
        return base;
    }
    let mut n: u32 = 2;
    loop {
        let id = format!("{base}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// Mint vehicle id using the supplied domain data.
pub(super) fn mint_vehicle_id(doc: &MissionDocCore, side: &str, j: usize) -> String {
    let existing = existing_vehicle_ids(doc);
    let base = format!("veh-{side}-apply-{j}");
    if !existing.contains(&base) {
        return base;
    }
    let mut n: u32 = 2;
    loop {
        let id = format!("{base}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// Existing slot ids using the supplied domain data.
pub(super) fn existing_slot_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.slots_json()) else {
        return std::collections::HashSet::new();
    };
    root.as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Existing vehicle ids using the supplied domain data.
pub(super) fn existing_vehicle_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return std::collections::HashSet::new();
    };
    root.get("vehiclesById")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Mint squad id using the supplied domain data.
pub(super) fn mint_squad_id(doc: &MissionDocCore, side: &str) -> String {
    let existing = existing_squad_ids(doc);
    let mut n: u32 = 1;
    loop {
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// Existing squad ids using the supplied domain data.
pub(super) fn existing_squad_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return std::collections::HashSet::new();
    };
    root.get("squadsById")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}
