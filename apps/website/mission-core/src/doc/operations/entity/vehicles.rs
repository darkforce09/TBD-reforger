//! Role: vehicles.
//! Position: `doc/operations/entity` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;
use super::NONE_IDX;
use super::OwnerOption;
use super::ensure_side_faction;

/// Domain representation of vehicle cargo row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VehicleCargoRow {
    /// Item.
    pub item: String,

    /// Qty.
    pub qty: i64,
}

/// Domain representation of vehicle row.
#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRow {
    /// Id.
    pub id: String,

    /// Resource name.
    pub resource_name: String,

    /// Xy.
    pub xy: Option<(f64, f64)>,

    /// Authored heading (degrees). `None` when unplaced; `Some(0.0)` is a real authored zero.
    pub rotation: Option<f64>,

    /// Elevation when placed; `None` when unplaced.
    pub z: Option<f64>,

    /// `faction-{SIDE}` when the vehicle was map-placed; empty when it only has a squad.
    pub faction_id: String,

    /// Empty when the vehicle is not attached to a squad (every map-placed vehicle).
    pub squad_id: String,

    /// Cargo.
    pub cargo: Vec<VehicleCargoRow>,

    /// Crew.
    pub crew: std::collections::HashMap<String, String>,
}

/// Apply set_vehicle_cargo to explicit document state.
pub fn set_vehicle_cargo(
    core: &MissionDocCore,
    vehicle_id: String,
    rows: Vec<VehicleCargoRow>,
) -> bool {
    let pairs: Vec<(String, i64)> = rows.into_iter().map(|r| (r.item, r.qty)).collect();
    core.set_vehicle_cargo(&vehicle_id, &pairs);
    true
}

/// Apply vehicle_rows to explicit document state.
pub fn vehicle_rows(core: &MissionDocCore) -> Vec<VehicleRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    let Some(map) = root.get("vehiclesById").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut rows: Vec<VehicleRow> = map
        .iter()
        .map(|(id, v)| {
            let s = |k: &str| {
                v.get(k)
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            let pos = v.get("position");
            VehicleRow {
                id: id.clone(),
                resource_name: s("resourceName"),
                xy: pos.and_then(|p| Some((p.get("x")?.as_f64()?, p.get("y")?.as_f64()?))),
                rotation: pos.and_then(|p| p.get("rotation")?.as_f64()),
                z: pos.and_then(|p| p.get("z")?.as_f64()),
                faction_id: s("factionId"),
                squad_id: s("squadId"),
                cargo: v
                    .get("cargo")
                    .and_then(|c| c.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|r| {
                                Some(VehicleCargoRow {
                                    item: r.get("item")?.as_str()?.to_string(),
                                    qty: r.get("qty")?.as_i64()?,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),

                crew: v
                    .get("crew")
                    .and_then(|c| c.as_object())
                    .map(|o| {
                        o.iter()
                            .filter_map(|(seat, slot)| {
                                Some((seat.clone(), slot.as_str()?.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect();

    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

/// Apply set_vehicle_heading to explicit document state.
pub fn set_vehicle_heading(core: &MissionDocCore, vehicle_id: String, heading_deg: f64) -> bool {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return false;
    };
    let Some(v) = root.get("vehiclesById").and_then(|m| m.get(&vehicle_id)) else {
        return false;
    };
    let Some(pos) = v.get("position") else {
        return false;
    };
    let (Some(x), Some(y)) = (
        pos.get("x").and_then(|n| n.as_f64()),
        pos.get("y").and_then(|n| n.as_f64()),
    ) else {
        return false;
    };
    let z = pos.get("z").and_then(|n| n.as_f64()).unwrap_or(0.0);
    core.set_vehicle_position(&vehicle_id, x, y, z, heading_deg);
    true
}

/// Apply placed_entity_pos to explicit document state.
pub fn placed_entity_pos(core: &MissionDocCore, id: &str) -> Option<(f64, f64)> {
    let soa = core.materialize();
    let row = soa.ids.iter().position(|s| s == id)?;
    Some((f64::from(soa.xs[row]), f64::from(soa.ys[row])))
}

/// `z`/`rotation` are `0.0` for the same reason the character path uses them: the flat-map commit has no DEM sample yet. Heading is authored afterwards, not guessed at drop.
pub fn place_vehicle_in_core(
    core: &MissionDocCore,
    side: &str,
    vehicle_id: &str,
    resource_name: &str,
    x: f64,
    y: f64,
    with_crew: bool,
) -> bool {
    if !matches!(side, "BLUFOR" | "OPFOR" | "INDFOR") || resource_name.trim().is_empty() {
        return false;
    }
    let faction_id = ensure_side_faction(core, side);

    core.place_vehicle_with_crew_stamp(
        vehicle_id,
        resource_name,
        x,
        y,
        0.0,
        0.0,
        &faction_id,
        with_crew,
    );
    true
}

/// Placed owner options using the supplied domain data.
#[must_use]
pub fn placed_owner_options(core: &MissionDocCore) -> Vec<OwnerOption> {
    let mut out: Vec<OwnerOption> = Vec::new();

    let soa = core.materialize();
    for (i, id) in soa.ids.iter().enumerate() {
        let role = {
            let idx = soa.role_idx.get(i).copied().unwrap_or(NONE_IDX);
            if idx == NONE_IDX {
                String::new()
            } else {
                soa.roles.get(idx as usize).cloned().unwrap_or_default()
            }
        };
        let label = if role.is_empty() {
            format!("Slot {id}")
        } else {
            format!("{role} ({id})")
        };
        out.push(OwnerOption {
            id: id.clone(),
            label,
        });
    }

    for v in vehicle_rows(core) {
        if v.xy.is_none() {
            continue;
        }
        let short = v
            .resource_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(&v.resource_name)
            .trim_end_matches(".et");
        let label = if short.is_empty() {
            format!("Vehicle {}", v.id)
        } else {
            format!("{short} ({})", v.id)
        };
        out.push(OwnerOption { id: v.id, label });
    }
    out.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    out
}
