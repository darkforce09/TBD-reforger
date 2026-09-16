//! Role: vehicles.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    CompileError, EntityIn, FactionIn, HashMap, ModEntity, ModEntityInventory, SquadIn, VehicleIn,
    is_wire_unsafe, render_authored_str, slug_key,
};

/// Normalize heading using the supplied domain data.
pub(super) fn normalize_heading(rotation: f64) -> f64 {
    if rotation.is_nan() || rotation.is_infinite() {
        return 0.0;
    }
    (rotation % 360.0 + 360.0) % 360.0
}

/// Drops rows with an empty `alias` (schema-required) or missing `position` (no honest x/z). `position.x → x`, `position.y → z`, `position.rotation → headingDeg` — the same locked mapping slots use. `faction` is emitted only when non-empty.
pub(super) fn derive_entities(rows: &[EntityIn]) -> Vec<ModEntity> {
    let mut out = Vec::new();
    for e in rows {
        let alias = e.alias.trim();
        if alias.is_empty() {
            continue;
        }
        let Some(pos) = e.position.as_ref() else {
            continue;
        };
        let faction = e.faction.trim();
        let heading = normalize_heading(pos.rotation);
        out.push(ModEntity {
            alias: alias.to_string(),
            uid: entity_uid(&e.id),
            x: pos.x,
            z: pos.y,
            heading_deg: Some(heading),
            faction: if faction.is_empty() {
                None
            } else {
                Some(faction.to_string())
            },
            inventory: Vec::new(),
        });
    }
    out
}

/// Map-placed vehicles store `factionId = "faction-BLUFOR"`; the schema wants `factionKey` `"blufor"`. Empty input → `None` (entity.faction is optional).
pub(super) fn faction_key_from_faction_id(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let stripped = lower.strip_prefix("faction-").unwrap_or(t);
    let key = slug_key(stripped, "");
    if key.is_empty() { None } else { Some(key) }
}

/// Derive vehicles as entities using the supplied domain data.
pub(super) fn derive_vehicles_as_entities(
    vehicles: &[VehicleIn],
    squads: &[SquadIn],
    factions: &[FactionIn],
    aliases: &crate::data::scenario::kit::KitAliases,
) -> Result<Vec<ModEntity>, CompileError> {
    let squads_by_id: HashMap<&str, &SquadIn> = squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let factions_by_id: HashMap<&str, &FactionIn> =
        factions.iter().map(|f| (f.id.as_str(), f)).collect();

    let mut out = Vec::new();
    for v in vehicles {
        let Some(pos) = v.position.as_ref() else {
            continue;
        };
        let Some(alias) = aliases.vehicle_for_resource(v.resource_name.trim()) else {
            return Err(CompileError::Parse(format!(
                "vehicle {}: resourceName {} has no veh: alias in kit-aliases.json \
                 (T-425 — refuse silent drop/substitute)",
                v.id, v.resource_name
            )));
        };

        let faction = vehicle_faction_key(v, &squads_by_id, &factions_by_id);

        out.push(ModEntity {
            alias: alias.to_string(),

            uid: roster_uid(v).unwrap_or(None),
            x: pos.x,
            z: pos.y,
            heading_deg: Some(normalize_heading(pos.rotation)),
            faction,
            inventory: vehicle_inventory(v),
        });
    }
    Ok(out)
}

/// Entity uid using the supplied domain data.
pub(super) fn entity_uid(id: &str) -> Option<String> {
    if id.trim().is_empty() || id.bytes().any(is_wire_unsafe) {
        return None;
    }
    Some(id.to_string())
}

/// Roster uid using the supplied domain data.
pub(super) fn roster_uid(v: &VehicleIn) -> Result<Option<String>, String> {
    if v.id.trim().is_empty() {
        return Ok(None);
    }
    if v.id.bytes().any(is_wire_unsafe) {
        return Err(format!(
            "its id {} carries a control character, which `wireSafeString` forbids — and repairing \
             an identity would mint a different one, which every reference to this vehicle would \
             then miss",
            render_authored_str(&v.id)
        ));
    }
    Ok(Some(v.id.clone()))
}

/// Same shared-derivation rule as [`vehicle_faction_key`] and [`roster_uid`]: the roster row and its `entities[]` twin describe ONE vehicle, so a second copy of "which cargo rows are real" would let them disagree about what is inside it. Blank items and non-positive quantities are not cargo.
pub(super) fn vehicle_inventory(v: &VehicleIn) -> Vec<ModEntityInventory> {
    v.cargo
        .iter()
        .filter(|r| !r.item.trim().is_empty() && r.qty >= 1)
        .map(|r| ModEntityInventory {
            item: r.item.clone(),
            qty: r.qty,
        })
        .collect()
}

/// `None` when neither path yields a key: `faction` is optional on both `$defs/entity` and `$defs/vehicle`.
pub(super) fn vehicle_faction_key(
    v: &VehicleIn,
    squads_by_id: &HashMap<&str, &SquadIn>,
    factions_by_id: &HashMap<&str, &FactionIn>,
) -> Option<String> {
    faction_key_from_faction_id(&v.faction_id).or_else(|| {
        let sq = squads_by_id.get(v.squad_id.trim())?;
        if let Some(f) = factions_by_id.get(sq.faction_id.as_str()) {
            let key = slug_key(&f.key, "");
            if key.is_empty() { None } else { Some(key) }
        } else {
            faction_key_from_faction_id(&sq.faction_id)
        }
    })
}
