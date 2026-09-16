//! Role: loadouts briefings.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    BTreeMap, FactionIn, MOD_MAX_MARKER_LABEL_CHARS, ModBriefing, ModMarker, ModSlotCargo,
    ModSlotGear, ModSlotLoadout, slug_key,
};

/// Mod slot loadout using the supplied domain data.
pub(super) fn mod_slot_loadout(lo: &serde_json::Value) -> Option<ModSlotLoadout> {
    let non_empty = |v: Option<&serde_json::Value>| {
        v.and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let wear = lo.get("wear");
    let wear_key = |k: &str| non_empty(wear.and_then(|w| w.get(k)));

    let mut gear = ModSlotGear {
        uniform: wear_key("jacket"),
        vest: wear_key("armoredVest").or_else(|| wear_key("vest")),
        helmet: wear_key("headCover"),
        pants: wear_key("pants"),
        boots: wear_key("boots"),
        handwear: wear_key("handwear"),
        backpack: wear_key("backpack"),
        ..ModSlotGear::default()
    };

    let weapons = lo.get("weapons").and_then(serde_json::Value::as_array);
    let weapon_at = |slot_index: i64, slot_type: &'static str| {
        weapons.and_then(|ws| {
            ws.iter().find(|w| {
                w.get("slotIndex").and_then(serde_json::Value::as_i64) == Some(slot_index)
                    && w.get("slotType").and_then(serde_json::Value::as_str) == Some(slot_type)
            })
        })
    };

    if let Some(primary) = weapon_at(0, "primary") {
        gear.primary = non_empty(primary.get("weapon"));

        gear.optic = non_empty(primary.get("optic"));
        gear.magazine = non_empty(primary.get("magazine"));

        gear.attachments = primary
            .get("attachments")
            .and_then(serde_json::Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|v| v.as_str().filter(|s| !s.is_empty()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
    }
    gear.launcher = weapon_at(1, "primary").and_then(|w| non_empty(w.get("weapon")));
    gear.handgun = weapon_at(2, "secondary").and_then(|w| non_empty(w.get("weapon")));
    gear.throwable = weapon_at(3, "grenade").and_then(|w| non_empty(w.get("weapon")));

    let cargo: Vec<ModSlotCargo> = lo
        .get("cargo")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    Some(ModSlotCargo {
                        container: non_empty(r.get("container"))?,
                        item: non_empty(r.get("item"))?,
                        qty: r
                            .get("qty")
                            .and_then(serde_json::Value::as_i64)
                            .filter(|q| *q >= 1)?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let gear = (!gear.is_empty()).then_some(gear);
    if gear.is_none() && cargo.is_empty() {
        return None;
    }
    Some(ModSlotLoadout { gear, cargo })
}

/// The row beats a sibling `briefings` map on a point that outlives convenience: the compiled map is `additionalProperties`-open, so an entry naming a faction the author later DELETED still validates, and the compile would ship orders to a side that no longer exists. Hanging prose on the row makes that state unrepresentable.
pub(super) fn derive_briefings(factions: &[FactionIn]) -> BTreeMap<String, ModBriefing> {
    let mut out: BTreeMap<String, ModBriefing> = BTreeMap::new();

    for f in factions {
        let Some(briefing) = f.briefing.as_ref() else {
            continue;
        };

        let entry = out.entry(slug_key(&f.key, "faction")).or_default();

        merge_prose(&mut entry.situation, briefing.situation.as_deref());
        merge_prose(&mut entry.mission, briefing.mission.as_deref());
        merge_prose(&mut entry.execution, briefing.execution.as_deref());

        for m in &briefing.markers {
            entry.markers.push(ModMarker {
                x: m.x,
                z: m.z,
                icon: m.icon.clone(),
                label: m.label.chars().take(MOD_MAX_MARKER_LABEL_CHARS).collect(),
            });
        }
    }

    out
}

/// Fold one authored prose field into a possibly-already-populated slot (the slug-collision case).
pub(super) fn merge_prose(slot: &mut Option<String>, authored: Option<&str>) {
    let Some(text) = authored else {
        return;
    };
    match slot {
        Some(existing) if !existing.is_empty() && !text.is_empty() => {
            existing.push_str("\n\n");
            existing.push_str(text);
        }
        Some(existing) if existing.is_empty() => *existing = text.to_string(),
        Some(_) => {}
        None => *slot = Some(text.to_string()),
    }
}
