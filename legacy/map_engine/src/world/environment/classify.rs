//! Role: classify.
//! Position: `world/environment` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use serde_json::Value;

/// Render-class wire codes. **The array index IS the `VisibleSet.classes` byte** (`worldObjectsCore.ts:42`) — this order is the wire format; do not reorder.
pub const RENDER_CLASS_CODES: [&str; 5] = ["building", "tree", "vegetation", "prop", "rockLarge"];

/// Unclassified sentinel (`NO_CLASS`, `worldObjectsCore.ts:268`) — never drawn or picked.
pub const NO_CLASS: u8 = 255;

/// Oversized-object half-extent threshold, meters (`OVERSIZED_HALF_EXTENT_M`, `:71`).
pub const OVERSIZED_HALF_EXTENT_M: f64 = 64.0;

/// Any kind that falls through to `_ => None` becomes `NO_CLASS` and is **never drawn or picked**. That is silent: no gate fails, the objects just vanish. Add an arm when adding a kind.
#[must_use]
pub fn render_class_for_prefab(kind: &str, cls: &str) -> Option<&'static str> {
    match kind {
        "building" => Some("building"),
        "water" => (cls == "pier" || cls == "dock").then_some("building"),
        "tree" => Some("tree"),
        "vegetation" => Some("vegetation"),
        "rock" => Some("rockLarge"),
        "prop" | "utility" | "vehicle" => Some("prop"),
        _ => None,
    }
}

/// `RENDER_CLASS_CODES.indexOf(name)` → code (or `NO_CLASS` for an unknown name).
#[must_use]
pub fn class_code(name: &str) -> u8 {
    RENDER_CLASS_CODES
        .iter()
        .position(|&c| c == name)
        .map_or(NO_CLASS, |i| i as u8)
}

#[must_use]
fn finite(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite())
}

/// `narrowInstanceRow(row)` (`:399`) → `(pid, x, y, z, rot)` or reject.
#[must_use]
pub fn narrow_instance_row(row: &Value) -> Option<(f64, f64, f64, f64, f64)> {
    let arr = row.as_array()?;
    if arr.len() < 3 {
        return None;
    }

    let pid = arr[0].as_f64()?;
    let x = finite(arr.get(1))?;
    let y = finite(arr.get(2))?;
    let z = finite(arr.get(3)).unwrap_or(0.0);
    let rot = finite(arr.get(4)).unwrap_or(0.0);
    Some((pid, x, y, z, rot))
}

/// Instance row v2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceRowV2 {
    /// Pid.
    pub pid: f64,

    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,

    /// Z.
    pub z: f64,

    /// Rot.
    pub rot: f64,

    /// Pitch.
    pub pitch: f64,

    /// Roll.
    pub roll: f64,

    /// Scale.
    pub scale: f64,
}

/// [`narrow_instance_row`] plus the optional trailing `pitchDeg, rollDeg, scale` (elements 5..8). The first five follow the v1 rule exactly (same accept / reject / default decisions); a trailer that is absent or non-finite takes its identity value, and a non-positive scale is identity too (a row can never collapse an object to nothing).
#[must_use]
pub fn narrow_instance_row_v2(row: &Value) -> Option<InstanceRowV2> {
    let (pid, x, y, z, rot) = narrow_instance_row(row)?;
    let arr = row.as_array()?;
    let pitch = finite(arr.get(5)).unwrap_or(0.0);
    let roll = finite(arr.get(6)).unwrap_or(0.0);
    let scale = finite(arr.get(7)).filter(|s| *s > 0.0).unwrap_or(1.0);
    Some(InstanceRowV2 {
        pid,
        x,
        y,
        z,
        rot,
        pitch,
        roll,
        scale,
    })
}

#[cfg(test)]
#[path = "tests/classify_tests.rs"]
mod tests;
