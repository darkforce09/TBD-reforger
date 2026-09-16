//! Role: position rows.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::Map;
use super::MapPrelim;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use super::RemintMap;
use super::TransactionMut;
use super::any_to_f64;
use super::json_str;
use super::slot_is_transform_locked;
use super::value_to_any;
use yrs::types::ToJson;

/// Position any using the supplied domain data.
pub(super) fn position_any(x: f64, y: f64, z: f64, rotation: f64) -> Any {
    position_any_merged(HashMap::new(), x, y, z, rotation)
}

/// Circle shape any using the supplied domain data.
pub(super) fn circle_shape_any(x: f64, z: f64, r: f64) -> Any {
    let circle: HashMap<String, Any> = HashMap::from([
        ("x".to_string(), Any::Number(x)),
        ("z".to_string(), Any::Number(z)),
        ("r".to_string(), Any::Number(r)),
    ]);
    Any::Map(Arc::new(HashMap::from([(
        "circle".to_string(),
        Any::Map(Arc::new(circle)),
    )])))
}

/// Polygon shape any using the supplied domain data.
pub(super) fn polygon_shape_any(points_flat: &[f64]) -> Any {
    let ring: Vec<Any> = points_flat
        .chunks_exact(2)
        .map(|p| Any::Array(vec![Any::Number(p[0]), Any::Number(p[1])].into()))
        .collect();
    Any::Map(Arc::new(HashMap::from([(
        "polygon".to_string(),
        Any::Array(ring.into()),
    )])))
}

/// Position any merged using the supplied domain data.
pub(super) fn position_any_merged(
    mut existing: HashMap<String, Any>,
    x: f64,
    y: f64,
    z: f64,
    rotation: f64,
) -> Any {
    existing.insert("x".to_string(), Any::Number(x));
    existing.insert("y".to_string(), Any::Number(y));
    existing.insert("z".to_string(), Any::Number(z));
    existing.insert("rotation".to_string(), Any::Number(rotation));
    Any::Map(Arc::new(existing))
}

/// Json position using the supplied domain data.
pub(super) fn json_position(
    m: &serde_json::Map<String, serde_json::Value>,
) -> (f64, f64, f64, f64) {
    let pos = m.get("position").and_then(serde_json::Value::as_object);
    let g = |k: &str| {
        pos.and_then(|p| p.get(k))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };
    (g("x"), g("y"), g("z"), g("rotation"))
}

/// Json position map using the supplied domain data.
pub(super) fn json_position_map(
    m: &serde_json::Map<String, serde_json::Value>,
) -> HashMap<String, Any> {
    match m.get("position") {
        Some(serde_json::Value::Object(pos)) => pos
            .iter()
            .map(|(k, v)| (k.clone(), value_to_any(v)))
            .collect(),
        _ => HashMap::new(),
    }
}

/// Offset shape any using the supplied domain data.
pub(super) fn offset_shape_any(shape: &serde_json::Value, dx: f64, dy: f64) -> Any {
    let Some(obj) = shape.as_object() else {
        return value_to_any(shape);
    };
    if let Some(circle) = obj.get("circle").and_then(serde_json::Value::as_object) {
        let g = |k: &str| {
            circle
                .get(k)
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
        };
        return circle_shape_any(g("x") + dx, g("z") + dy, g("r"));
    }
    if let Some(poly) = obj.get("polygon").and_then(serde_json::Value::as_array) {
        let ring: Vec<Any> = poly
            .iter()
            .filter_map(serde_json::Value::as_array)
            .filter(|p| p.len() == 2)
            .map(|p| {
                let x = p[0].as_f64().unwrap_or(0.0) + dx;
                let z = p[1].as_f64().unwrap_or(0.0) + dy;
                Any::Array(vec![Any::Number(x), Any::Number(z)].into())
            })
            .collect();
        return Any::Map(Arc::new(HashMap::from([(
            "polygon".to_string(),
            Any::Array(ring.into()),
        )])));
    }
    value_to_any(shape)
}

/// Merge shape rows using the supplied domain data.
#[allow(clippy::too_many_arguments)]
pub(super) fn merge_shape_rows(
    txn: &mut TransactionMut,
    map: &MapRef,
    rows: Option<&serde_json::Value>,
    remint: &RemintMap,
    dx: f64,
    dy: f64,
    kind: &str,
    skipped: &mut Vec<(String, String, String)>,
) -> u32 {
    let mut added = 0;
    let Some(rows) = rows.and_then(serde_json::Value::as_array) else {
        return 0;
    };
    for row in rows {
        let Some(m) = row.as_object() else {
            skipped.push((kind.to_string(), String::new(), "not an object".to_string()));
            continue;
        };
        let Some(old_id) = json_str(m, "id") else {
            skipped.push((kind.to_string(), String::new(), "missing id".to_string()));
            continue;
        };
        let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
        let entity = map.insert(
            txn,
            new_id.as_str(),
            MapPrelim::from([("id", new_id.as_str())]),
        );
        for (k, v) in m {
            match k.as_str() {
                "id" => {}
                "shape" => {
                    entity.insert(txn, "shape", offset_shape_any(v, dx, dy));
                }
                _ => {
                    entity.insert(txn, k.as_str(), value_to_any(v));
                }
            };
        }
        added += 1;
    }
    added
}

/// Update vehicle position in txn using the supplied domain data.
pub(super) fn update_vehicle_position_in_txn(
    txn: &mut TransactionMut,
    vehicles: &MapRef,
    id: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) -> bool {
    let Some(Out::YMap(v)) = vehicles.get(&*txn, id) else {
        return false;
    };
    let (mut px, mut py, mut pz, mut prot) = read_position(txn, &v);
    if let Some(nx) = x.filter(|v| v.is_finite()) {
        px = nx;
    }
    if let Some(ny) = y.filter(|v| v.is_finite()) {
        py = ny;
    }
    if let Some(nz) = z.filter(|v| v.is_finite()) {
        pz = nz;
    }
    if let Some(nr) = rotation.filter(|v| v.is_finite()) {
        prot = ((nr % 360.0) + 360.0) % 360.0;
    }
    let existing = read_position_map(txn, &v);
    v.insert(
        &mut *txn,
        "position",
        position_any_merged(existing, px, py, pz, prot),
    );
    true
}

/// Move entities in txn using the supplied domain data.
pub(super) fn move_entities_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    editor_layers: &MapRef,
    ids: &[String],
    dx: f64,
    dy: f64,
    zs: &[f64],
) {
    for (i, id) in ids.iter().enumerate() {
        if slot_is_transform_locked(&*txn, editor_layers, id) {
            continue;
        }
        if let Some(Out::YMap(slot)) = slots.get(&*txn, id.as_str()) {
            let (px, py, _pz, prot) = read_position(txn, &slot);
            let z = zs.get(i).copied().unwrap_or(0.0);
            let existing = read_position_map(txn, &slot);
            slot.insert(
                &mut *txn,
                "position",
                position_any_merged(existing, px + dx, py + dy, z, prot),
            );
        }
    }
}

/// Move vehicles in txn using the supplied domain data.
pub(super) fn move_vehicles_in_txn(
    txn: &mut TransactionMut,
    vehicles: &MapRef,
    ids: &[String],
    dx: f64,
    dy: f64,
) {
    for id in ids {
        let Some(Out::YMap(v)) = vehicles.get(&*txn, id.as_str()) else {
            continue;
        };
        if v.get(&*txn, "position").is_none() {
            continue;
        }
        let (px, py, pz, prot) = read_position(txn, &v);
        let existing = read_position_map(txn, &v);
        v.insert(
            &mut *txn,
            "position",
            position_any_merged(existing, px + dx, py + dy, pz, prot),
        );
    }
}

/// Read composition map using the supplied domain data.
pub(super) fn read_composition_map<T: ReadTxn>(
    txn: &T,
    compositions: &MapRef,
    id: &str,
) -> Option<HashMap<String, Any>> {
    match compositions.get(txn, id) {
        Some(Out::Any(Any::Map(m))) => Some((*m).clone()),
        Some(Out::YMap(row)) => Some(
            row.iter(txn)
                .map(|(k, out)| match out {
                    Out::Any(a) => (k.to_string(), a),

                    other => (k.to_string(), other.to_json(txn)),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Read position using the supplied domain data.
pub(super) fn read_position<T: ReadTxn>(txn: &T, slot: &MapRef) -> (f64, f64, f64, f64) {
    if let Some(Out::Any(Any::Map(m))) = slot.get(txn, "position") {
        let g = |k: &str| m.get(k).map_or(0.0, any_to_f64);
        (g("x"), g("y"), g("z"), g("rotation"))
    } else {
        (0.0, 0.0, 0.0, 0.0)
    }
}

/// Read position map using the supplied domain data.
pub(super) fn read_position_map<T: ReadTxn>(txn: &T, slot: &MapRef) -> HashMap<String, Any> {
    match slot.get(txn, "position") {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        _ => HashMap::new(),
    }
}
