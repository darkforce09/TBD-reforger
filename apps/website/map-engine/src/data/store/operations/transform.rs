//! Role: transform.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::attrs::{keep_z_rows, slot_z};
use super::entity::terrain_bounds_of;
use crate::data::store::{EntityTransformPatch, MissionDocCore};

/// One selected entity resolved to its kind + current world position, for the placement math.
pub struct SelPos {
    id: String,

    is_slot: bool,
    x: f64,
    y: f64,

    z: f64,
}

/// Resolve selection positions using the supplied domain data.
pub fn resolve_selection_positions(
    core: &MissionDocCore,
    sel: &[String],
) -> (Vec<SelPos>, [f64; 4]) {
    let soa = core.materialize();
    let veh_root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
    let tb = terrain_bounds_of(core);
    let mut out = Vec::with_capacity(sel.len());
    for id in sel {
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            out.push(SelPos {
                id: id.clone(),
                is_slot: true,
                x: f64::from(soa.xs[row]),
                y: f64::from(soa.ys[row]),
                z: f64::from(soa.zs[row]),
            });
            continue;
        }
        if let Some(pos) = veh_root
            .as_ref()
            .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
        {
            let (Some(vx), Some(vy)) = (
                pos.get("x").and_then(serde_json::Value::as_f64),
                pos.get("y").and_then(serde_json::Value::as_f64),
            ) else {
                continue;
            };
            out.push(SelPos {
                id: id.clone(),
                is_slot: false,
                x: vx,
                y: vy,
                z: pos
                    .get("z")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            });
        }
    }
    (out, tb)
}

/// The rows are read ONCE for the whole batch, not per entity: this commits `k` entities and [`raw_slot_rows`] is an O(document) JSON parse. `keep_z_rows` is asked with the FIRST moved slot's write shape (x and y set, z absent).
pub fn commit_positions(
    core: &MissionDocCore,
    entities: &[SelPos],
    targets: &[super::placement::Pt],
    tb: [f64; 4],
) -> bool {
    let z_rows = entities
        .iter()
        .zip(targets.iter())
        .find(|(e, t)| e.is_slot && (e.x != t.x || e.y != t.y))
        .and_then(|(_, t)| keep_z_rows(core, Some(t.x), Some(t.y), None));
    let mut patches: Vec<EntityTransformPatch> = Vec::new();
    for (e, t) in entities.iter().zip(targets.iter()) {
        if e.x == t.x && e.y == t.y {
            continue;
        }
        if e.is_slot {
            let z = z_rows.as_ref().and_then(|rows| slot_z(rows, &e.id));
            patches.push(EntityTransformPatch {
                id: e.id.clone(),
                is_slot: true,
                x: Some(t.x),
                y: Some(t.y),
                z,
                rotation: None,
            });
        } else {
            let heading = vehicle_heading_of(core, &e.id).unwrap_or(0.0);
            patches.push(EntityTransformPatch {
                id: e.id.clone(),
                is_slot: false,
                x: Some(t.x.clamp(0.0, tb[2])),
                y: Some(t.y.clamp(0.0, tb[3])),
                z: Some(e.z),
                rotation: Some(heading),
            });
        }
    }
    core.update_entity_transforms(&patches, tb[2], tb[3]) > 0
}

/// Read a vehicle's current heading (rotation) off `small_maps_json`; `None` if absent/unplaced.
pub fn vehicle_heading_of(core: &MissionDocCore, id: &str) -> Option<f64> {
    let root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok()?;
    root.get("vehiclesById")?
        .get(id)?
        .get("position")?
        .get("rotation")?
        .as_f64()
}

/// Apply rotate_selection_to_face to explicit document state.
pub fn rotate_selection_to_face(
    core: &MissionDocCore,
    cx: f64,
    cy: f64,
    rung: usize,
    sel: Vec<String>,
) -> bool {
    let soa = core.materialize();
    let veh_root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
    let terrain = veh_root
        .as_ref()
        .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
        .unwrap_or_default();
    let tb = crate::data::scenario::compile::terrain_bounds(&terrain);
    let mut items: Vec<(String, bool, f64)> = Vec::new();
    for id in &sel {
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            let (sx, sy) = (f64::from(soa.xs[row]), f64::from(soa.ys[row]));
            if let Some(bearing) = super::rotation::bearing_to_face(sx, sy, cx, cy) {
                let deg = super::rotation::snap_rotate(bearing, rung);
                items.push((id.clone(), true, deg));
            }
            continue;
        }
        let Some(pos) = veh_root
            .as_ref()
            .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
        else {
            continue;
        };
        let (Some(vx), Some(vy)) = (
            pos.get("x").and_then(serde_json::Value::as_f64),
            pos.get("y").and_then(serde_json::Value::as_f64),
        ) else {
            continue;
        };
        if let Some(bearing) = super::rotation::bearing_to_face(vx, vy, cx, cy) {
            let deg = super::rotation::snap_rotate(bearing, rung);
            items.push((id.clone(), false, deg));
        }
    }
    core.rotate_entities(&items, tb[2], tb[3]) > 0
}

/// Apply apply_pattern_to_selection to explicit document state.
pub fn apply_pattern_to_selection(
    core: &MissionDocCore,
    kind: super::placement::PatternKind,
    sel: Vec<String>,
    confirm_bulk: impl Fn(usize, &str) -> bool,
) -> bool {
    let (entities, tb) = resolve_selection_positions(core, &sel);
    if entities.len() < 2 {
        return false;
    }
    let src: Vec<super::placement::Pt> = entities
        .iter()
        .map(|e| super::placement::Pt::new(e.x, e.y))
        .collect();
    let targets = match kind {
        super::placement::PatternKind::Circular => super::placement::pattern_circular(&src),
        super::placement::PatternKind::Line => super::placement::pattern_line(&src),
        super::placement::PatternKind::Grid => super::placement::pattern_grid(&src),
        super::placement::PatternKind::FillArea => {
            let ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();
            let seed = super::placement::seed_from_ids(&ids);
            super::placement::pattern_fill_area(&src, seed)
        }
    };
    if !confirm_bulk(
        entities.len(),
        &format!("apply the {} pattern to", kind.label()),
    ) {
        return false;
    }
    commit_positions(core, &entities, &targets, tb)
}

/// Apply align_selection to explicit document state.
pub fn align_selection(
    core: &MissionDocCore,
    edge: super::placement::AlignEdge,
    sel: Vec<String>,
    confirm_bulk: impl Fn(usize, &str) -> bool,
) -> bool {
    let (entities, tb) = resolve_selection_positions(core, &sel);
    if entities.len() < 2 {
        return false;
    }
    let src: Vec<super::placement::Pt> = entities
        .iter()
        .map(|e| super::placement::Pt::new(e.x, e.y))
        .collect();
    let targets = super::placement::align_edge(&src, edge);
    if !confirm_bulk(entities.len(), "align") {
        return false;
    }
    commit_positions(core, &entities, &targets, tb)
}

/// Apply space_selection to explicit document state.
pub fn space_selection(
    core: &MissionDocCore,
    axis: super::placement::SpaceAxis,
    sel: Vec<String>,
    confirm_bulk: impl Fn(usize, &str) -> bool,
) -> bool {
    let (entities, tb) = resolve_selection_positions(core, &sel);
    if entities.len() < 3 {
        return false;
    }
    let src: Vec<super::placement::Pt> = entities
        .iter()
        .map(|e| super::placement::Pt::new(e.x, e.y))
        .collect();
    let targets = super::placement::space_equally(&src, axis);
    if !confirm_bulk(entities.len(), "space") {
        return false;
    }
    commit_positions(core, &entities, &targets, tb)
}

/// Apply orient_selection to explicit document state.
pub fn orient_selection(
    core: &MissionDocCore,
    cmd: super::placement::Orient,
    sel: Vec<String>,
    confirm_bulk: impl Fn(usize, &str) -> bool,
) -> bool {
    let (entities, tb) = resolve_selection_positions(core, &sel);
    if entities.is_empty() {
        return false;
    }
    if !confirm_bulk(entities.len(), "re-orient") {
        return false;
    }
    let pivot = super::placement::centroid(
        &entities
            .iter()
            .map(|e| super::placement::Pt::new(e.x, e.y))
            .collect::<Vec<_>>(),
    );
    let mut items: Vec<(String, bool, f64)> = Vec::new();
    for e in &entities {
        let Some(deg) =
            super::placement::orient_yaw(cmd, super::placement::Pt::new(e.x, e.y), pivot)
        else {
            continue;
        };
        items.push((e.id.clone(), e.is_slot, deg));
    }
    core.rotate_entities(&items, tb[2], tb[3]) > 0
}
