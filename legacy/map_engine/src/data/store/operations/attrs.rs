//! Role: attrs.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::entity::terrain_bounds_of;
use crate::data::store::{EntityTransformPatch, MissionDocCore, NONE_IDX};

/// One slot's editable attributes for the Attributes modal.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotAttrs {
    /// Id.
    pub id: String,

    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,

    /// Z.
    pub z: f64,

    /// Rotation.
    pub rotation: f64,

    /// Stance.
    pub stance: String,

    /// Role.
    pub role: String,

    /// Tag.
    pub tag: String,

    /// Squad.
    pub squad: String,

    /// Asset id.
    pub asset_id: String,

    /// Description.
    pub description: String,
}

/// The raw rows, NOT the SoA: `assetId` and `description` (and every other authored key) live only here. Parsed once per call and handed to the readers below, because `slots_json` is O(all slots) JSON and the modal must not pay it per field. Both callers already pay one `materialize()` of the same order, and both run on a modal render — never the frame loop.
pub fn raw_slot_rows(core: &MissionDocCore) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(&core.slots_json()) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    }
}

/// Row str using the supplied domain data.
pub fn row_str(rows: &serde_json::Map<String, serde_json::Value>, id: &str, key: &str) -> String {
    rows.get(id)
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Slot z using the supplied domain data.
pub fn slot_z(rows: &serde_json::Map<String, serde_json::Value>, id: &str) -> Option<f64> {
    rows.get(id)?
        .get("position")?
        .get("z")?
        .as_f64()
        .filter(|v| v.is_finite())
}

/// The fix is here, at the CALLER, not in the mutator: `MissionDocCore::update_slot_position` claims byte-parity with `ydoc.updateSlotPosition` and keeps it. Its callers read the current `z` and pass it back in, which makes the terrain-follow a no-op for the paths that have no sampler behind them.
pub fn keep_z_rows(
    core: &MissionDocCore,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    (z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))
}

/// Slot attrs from raw using the supplied domain data.
pub fn slot_attrs_from_raw(
    rows: &serde_json::Map<String, serde_json::Value>,
    id: &str,
) -> SlotAttrs {
    let pos = rows.get(id).and_then(|r| r.get("position"));
    let num = |key: &str| -> f64 {
        pos.and_then(|p| p.get(key))
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0)
    };

    let z = slot_z(rows, id).unwrap_or_else(|| num("z"));
    let stance_raw = row_str(rows, id, "stance");
    let stance = match stance_raw.as_str() {
        "crouch" => "crouch",
        "prone" => "prone",
        _ => "stand",
    };
    SlotAttrs {
        id: id.to_string(),
        x: num("x"),
        y: num("y"),
        z,
        rotation: num("rotation"),
        stance: stance.to_string(),
        role: row_str(rows, id, "role"),
        tag: row_str(rows, id, "tag"),
        squad: row_str(rows, id, "squadId"),
        asset_id: row_str(rows, id, "assetId"),
        description: row_str(rows, id, "description"),
    }
}

/// Eden's multi-edit rule has two halves. This is the first: a field whose value is identical on every selected entity can show that value; a field whose values differ has no value to show, so the modal blanks it and disables it until its per-field checkbox opts it in. `attributes.rs` owns the second half (the checkbox + the disable).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttrDiff {
    /// X.
    pub x: bool,

    /// Y.
    pub y: bool,

    /// Z.
    pub z: bool,

    /// Rotation.
    pub rotation: bool,

    /// Stance.
    pub stance: bool,

    /// Role.
    pub role: bool,

    /// Tag.
    pub tag: bool,

    /// Asset id.
    pub asset_id: bool,

    /// Description.
    pub description: bool,
}

impl AttrDiff {
    /// True when at least one field disagrees — the modal's "Multiple values" hint.
    #[must_use]
    pub fn any(self) -> bool {
        self.x
            || self.y
            || self.z
            || self.rotation
            || self.stance
            || self.role
            || self.tag
            || self.asset_id
            || self.description
    }
}

/// Apply read_attrs to explicit document state.
pub fn read_attrs(core: &MissionDocCore, id: &str) -> Option<SlotAttrs> {
    let rows = raw_slot_rows(core);

    if !rows.contains_key(id) {
        return None;
    }
    let soa = core.materialize();
    if let Some(row) = soa.ids.iter().position(|s| s == id) {
        let dict = |idx: u32, dict: &[String]| {
            if idx == NONE_IDX {
                String::new()
            } else {
                dict.get(idx as usize).cloned().unwrap_or_default()
            }
        };
        let stance = match soa.stance.get(row).copied().unwrap_or(0) {
            crate::data::store::STANCE_CROUCH => "crouch",
            crate::data::store::STANCE_PRONE => "prone",
            _ => "stand",
        };
        Some(SlotAttrs {
            id: id.to_string(),
            x: f64::from(soa.xs[row]),
            y: f64::from(soa.ys[row]),
            z: f64::from(soa.zs[row]),
            rotation: f64::from(soa.rotations[row]),
            stance: stance.to_string(),
            role: dict(soa.role_idx[row], &soa.roles),
            tag: dict(soa.tag_idx[row], &soa.tags),
            squad: dict(soa.squad_idx[row], &soa.squads),
            asset_id: row_str(&rows, id, "assetId"),
            description: row_str(&rows, id, "description"),
        })
    } else {
        Some(slot_attrs_from_raw(&rows, id))
    }
}

/// Apply attrs_locked_count to explicit document state.
pub fn attrs_locked_count(core: &MissionDocCore, ids: &[String]) -> usize {
    ids.iter()
        .filter(|id| core.slot_layer_is_locked(id))
        .count()
}

/// Apply attrs_update_position to explicit document state.
pub fn attrs_update_position(
    core: &MissionDocCore,
    id: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) -> bool {
    if core.slot_layer_is_locked(id) {
        return false;
    }

    let z = z.or_else(|| keep_z_rows(core, x, y, z).and_then(|rows| slot_z(&rows, id)));
    let b = terrain_bounds_of(core);
    core.update_slot_position(id, x, y, z, rotation, b[2], b[3]);
    true
}

/// Apply attrs_update_position_multi to explicit document state.
pub fn attrs_update_position_multi(
    core: &MissionDocCore,
    ids: &[String],
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) -> bool {
    let b = terrain_bounds_of(core);

    let rows = keep_z_rows(core, x, y, z);

    let mut patches: Vec<EntityTransformPatch> = Vec::with_capacity(ids.len());
    for id in ids {
        if core.slot_layer_is_locked(id) {
            continue;
        }
        let z = z.or_else(|| rows.as_ref().and_then(|r| slot_z(r, id)));
        patches.push(EntityTransformPatch {
            id: id.clone(),
            is_slot: true,
            x,
            y,
            z,
            rotation,
        });
    }
    core.update_entity_transforms(&patches, b[2], b[3]) > 0
}

/// Apply attrs_update_slot to explicit document state.
pub fn attrs_update_slot(
    core: &MissionDocCore,
    id: &str,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) -> bool {
    if !raw_slot_rows(core).contains_key(id) {
        return false;
    }
    if role.is_some() || tag.is_some() || stance.is_some() {
        core.update_slot(id, role, tag, stance);
    }
    core.update_slot_object(id, asset_id, description);
    true
}

/// Apply attrs_update_slot_multi to explicit document state.
#[expect(
    clippy::too_many_arguments,
    reason = "Preserve the editor's field-wise patch interface with an explicit document"
)]
pub fn attrs_update_slot_multi(
    core: &MissionDocCore,
    ids: &[String],
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
    slot_half: bool,
) -> bool {
    core.update_slots_attr_batch(
        ids,
        slot_half,
        role.clone(),
        tag.clone(),
        stance.clone(),
        asset_id.clone(),
        description.clone(),
    );
    true
}

/// Apply read_attrs_diff to explicit document state.
pub fn read_attrs_diff(core: &MissionDocCore, ids: &[String]) -> AttrDiff {
    let soa = core.materialize();

    let rows: Vec<(&String, usize)> = ids
        .iter()
        .filter_map(|id| Some((id, soa.ids.iter().position(|s| s == id)?)))
        .collect();
    let Some((&(first_id, first), rest)) = rows.split_first() else {
        return AttrDiff::default();
    };
    let raw = raw_slot_rows(core);

    let text = |idx: u32, dict: &[String]| {
        if idx == NONE_IDX {
            String::new()
        } else {
            dict.get(idx as usize).cloned().unwrap_or_default()
        }
    };
    let mut d = AttrDiff::default();
    for &(id, r) in rest {
        d.x |= soa.xs[r].to_bits() != soa.xs[first].to_bits();
        d.y |= soa.ys[r].to_bits() != soa.ys[first].to_bits();
        d.z |= soa.zs[r].to_bits() != soa.zs[first].to_bits();
        d.rotation |= soa.rotations[r].to_bits() != soa.rotations[first].to_bits();
        d.stance |=
            soa.stance.get(r).copied().unwrap_or(0) != soa.stance.get(first).copied().unwrap_or(0);
        d.role |= text(soa.role_idx[r], &soa.roles) != text(soa.role_idx[first], &soa.roles);
        d.tag |= text(soa.tag_idx[r], &soa.tags) != text(soa.tag_idx[first], &soa.tags);

        d.asset_id |= row_str(&raw, id, "assetId") != row_str(&raw, first_id, "assetId");
        d.description |= row_str(&raw, id, "description") != row_str(&raw, first_id, "description");
    }
    d
}
