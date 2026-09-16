//! Role: placement.
//! Position: `doc/operations/place_orbat` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;
use super::Value;

/// Canonical valid sides value.
pub(super) const VALID_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

/// Error from [`place_character_under_side`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceOrbatError {
    /// `side` was not BLUFOR / OPFOR / INDFOR.
    InvalidSide(String),
}

impl std::fmt::Display for PlaceOrbatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSide(s) => write!(f, "invalid side {s:?}; expected BLUFOR|OPFOR|INDFOR"),
        }
    }
}

impl std::error::Error for PlaceOrbatError {}

/// Ensures `faction-{SIDE}` exists (`id` + `key` + `name` = SIDE), then files the slot into that side's **current squad** — [`current_squad`], minting one only when there is no open squad to join. The slot is appended at the end of `slotIds` and becomes `leaderSlotId` only if it is the squad's first body, so a placement never steals the SL from a squad already led.
#[allow(clippy::too_many_arguments)]
pub fn place_character_under_side(
    doc: &MissionDocCore,
    side: &str,
    slot_id: &str,
    layer_id: &str,
    role: &str,
    tag: Option<String>,
    asset_id: Option<String>,
    x: f64,
    y: f64,
    z: f64,
    rotation: f64,
) -> Result<(String, String, String), PlaceOrbatError> {
    if !VALID_SIDES.contains(&side) {
        return Err(PlaceOrbatError::InvalidSide(side.to_string()));
    }

    let faction_id = format!("faction-{side}");
    ensure_side_faction(doc, side, &faction_id);

    let squad_id = match current_squad(doc, &faction_id) {
        Some(open) => open,
        None => {
            let id = mint_squad_id(doc, side);
            let squad_name = format!("Squad {}", squad_ordinal(doc, &faction_id) + 1);
            doc.add_squad(&id, &faction_id, &squad_name, None);
            id
        }
    };

    let existing = squad_slot_ids(doc, &squad_id);
    doc.add_slot(
        slot_id,
        &squad_id,
        layer_id,
        u32::try_from(existing.len()).unwrap_or(u32::MAX),
        role,
        tag,
        asset_id,
        x,
        y,
        z,
        rotation,
    );
    if existing.is_empty() {
        doc.set_leader(&squad_id, slot_id);
    }
    Ok((faction_id, squad_id, slot_id.to_string()))
}

/// Current squad using the supplied domain data.
pub(super) fn current_squad(doc: &MissionDocCore, faction_id: &str) -> Option<String> {
    let root = serde_json::from_str::<Value>(&doc.small_maps_json()).ok()?;
    let last = root
        .get("factionsById")?
        .get(faction_id)?
        .get("squadIds")?
        .as_array()?
        .last()?
        .as_str()?;
    let squad = root.get("squadsById")?.get(last)?;
    is_open_for_placement(squad).then(|| last.to_string())
}

/// Is open for placement using the supplied domain data.
pub(super) fn is_open_for_placement(squad: &Value) -> bool {
    let name = squad
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let callsign = squad
        .get("callsign")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let vehicles = squad
        .get("vehicleIds")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    is_minted_squad_name(name) && callsign.is_empty() && vehicles == 0
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
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Ensure side faction using the supplied domain data.
pub(super) fn ensure_side_faction(doc: &MissionDocCore, side: &str, faction_id: &str) {
    if faction_exists(doc, faction_id) {
        return;
    }
    doc.add_faction(faction_id, side, side);
}

/// Faction exists using the supplied domain data.
pub(super) fn faction_exists(doc: &MissionDocCore, faction_id: &str) -> bool {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return false;
    };
    root.get("factionsById")
        .and_then(|v| v.as_object())
        .is_some_and(|m| m.contains_key(faction_id))
}

/// Squad ordinal using the supplied domain data.
pub(super) fn squad_ordinal(doc: &MissionDocCore, faction_id: &str) -> usize {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return 0;
    };
    root.get("factionsById")
        .and_then(|v| v.get(faction_id))
        .and_then(|f| f.get("squadIds"))
        .and_then(|a| a.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
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
