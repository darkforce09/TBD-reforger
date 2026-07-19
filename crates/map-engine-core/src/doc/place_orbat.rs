//! T-180.1 — place a character under a side faction, minting a new squad with the slot as leader.
//!
//! Pure helper over [`MissionDocCore`] so Class-R gates run as native `cargo test` (no wasm).

use serde_json::Value;

use super::MissionDocCore;

/// Valid Eden sides for T-180 place (no CIV).
const VALID_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

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

/// Ensures `faction-{SIDE}` exists (`id` + `key` + `name` = SIDE). Mints a unique squad under it,
/// adds the slot as sole member (`index` 0), and sets `leaderSlotId` to that slot.
///
/// Returns `(faction_id, squad_id, slot_id)`.
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
    let squad_id = mint_squad_id(doc, side);
    let squad_name = format!("Squad {}", squad_ordinal(doc, &faction_id) + 1);
    doc.add_squad(&squad_id, &faction_id, &squad_name, None);
    doc.add_slot(
        slot_id, &squad_id, layer_id, 0, role, tag, asset_id, x, y, z, rotation,
    );
    doc.set_leader(&squad_id, slot_id);
    Ok((faction_id, squad_id, slot_id.to_string()))
}

fn ensure_side_faction(doc: &MissionDocCore, side: &str, faction_id: &str) {
    if faction_exists(doc, faction_id) {
        return;
    }
    doc.add_faction(faction_id, side, side);
}

fn faction_exists(doc: &MissionDocCore, faction_id: &str) -> bool {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return false;
    };
    root.get("factionsById")
        .and_then(|v| v.as_object())
        .is_some_and(|m| m.contains_key(faction_id))
}

fn squad_ordinal(doc: &MissionDocCore, faction_id: &str) -> usize {
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

fn mint_squad_id(doc: &MissionDocCore, side: &str) -> String {
    let existing = existing_squad_ids(doc);
    let mut n: u32 = 1;
    loop {
        // Format never equals the retired dump id `squad-1` (A1 / A-L2).
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

fn existing_squad_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return std::collections::HashSet::new();
    };
    root.get("squadsById")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn layer(doc: &MissionDocCore) {
        doc.add_editor_layer("lyr", "Layer 1", None);
    }

    fn place(
        doc: &MissionDocCore,
        side: &str,
        slot: &str,
    ) -> Result<(String, String, String), PlaceOrbatError> {
        place_character_under_side(
            doc,
            side,
            slot,
            "lyr",
            "Rifleman",
            None,
            Some("asset/Rifleman.et".into()),
            100.0,
            200.0,
            0.0,
            0.0,
        )
    }

    fn small(doc: &MissionDocCore) -> Value {
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
    }

    fn slots(doc: &MissionDocCore) -> Value {
        serde_json::from_str(&doc.slots_json()).expect("slots_json")
    }

    /// A1 — OPFOR place mints faction-OPFOR + new squad (≠ squad-1) with leader = slot.
    #[test]
    fn place_character_under_side_opfor() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let (fid, sid, slot) = place(&doc, "OPFOR", "n0").expect("place");
        assert_eq!(fid, "faction-OPFOR");
        assert_eq!(slot, "n0");
        assert_ne!(sid, "squad-1", "must not dump into squad-1");

        let root = small(&doc);
        let faction = &root["factionsById"]["faction-OPFOR"];
        assert_eq!(faction["key"], "OPFOR");
        assert_eq!(faction["name"], "OPFOR");
        let squad_ids = faction["squadIds"].as_array().expect("squadIds");
        assert_eq!(squad_ids.len(), 1);
        assert_eq!(squad_ids[0], sid);

        let squad = &root["squadsById"][&sid];
        let slot_ids = squad["slotIds"].as_array().expect("slotIds");
        assert_eq!(slot_ids, &vec![Value::String("n0".into())]);
        assert_eq!(squad["leaderSlotId"], "n0");
    }

    /// A2 — callsign + rank round-trip via update_slot_identity + slots_json.
    #[test]
    fn slot_callsign_rank_roundtrip() {
        let doc = MissionDocCore::new();
        layer(&doc);
        place(&doc, "BLUFOR", "n1").expect("place");
        doc.update_slot_identity("n1", Some("Alpha-1".into()), Some("Sergeant".into()));
        let v = slots(&doc);
        assert_eq!(v["n1"]["callsign"], "Alpha-1");
        assert_eq!(v["n1"]["rank"], "Sergeant");

        // Clear both keys.
        doc.update_slot_identity("n1", None, None);
        let v = slots(&doc);
        assert!(v["n1"].get("callsign").is_none(), "{v}");
        assert!(v["n1"].get("rank").is_none(), "{v}");
    }

    /// A4 — two places same side ⇒ two distinct squads under one faction.
    #[test]
    fn two_places_two_squads_same_side() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let (_, s1, _) = place(&doc, "BLUFOR", "a").expect("p1");
        let (_, s2, _) = place(&doc, "BLUFOR", "b").expect("p2");
        assert_ne!(s1, s2);
        let root = small(&doc);
        let squad_ids = root["factionsById"]["faction-BLUFOR"]["squadIds"]
            .as_array()
            .expect("squadIds");
        assert_eq!(squad_ids.len(), 2);
        assert!(squad_ids.iter().any(|v| v == &s1));
        assert!(squad_ids.iter().any(|v| v == &s2));
    }

    /// A5 — invalid side ⇒ Err and no mutation.
    #[test]
    fn place_rejects_invalid_side() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let before_small = small(&doc);
        let before_slots = slots(&doc);
        for bad in ["CIV", "nope"] {
            let err = place(&doc, bad, "x").expect_err("must reject");
            assert!(matches!(err, PlaceOrbatError::InvalidSide(_)), "{err:?}");
        }
        // Compare parsed Values — yrs `to_json` map key order is not stable across calls.
        assert_eq!(small(&doc), before_small);
        assert_eq!(slots(&doc), before_slots);
        assert!(
            before_small["factionsById"]
                .as_object()
                .is_some_and(|m| m.is_empty()),
            "no faction minted on reject"
        );
        assert!(
            before_slots.as_object().is_some_and(|m| m.is_empty()),
            "no slot minted on reject"
        );
    }
}
