//! The render classes a prefab is drawn and picked as.
//!
//! **Role:** maps a prefab's `kind` and `class` to its render class ([`render_class_for_prefab`])
//! and that class to its one-byte wire code ([`class_code`], [`RENDER_CLASS_CODES`]), and narrows
//! a chunk's raw instance rows ([`narrow_instance_row`], [`narrow_instance_row_v2`]).
//! **Position:** the bottom of `prefab_catalog`; read by [`crate::prefab_rows`], by `world_chunks`
//! when it decodes a chunk, by the map engine's draw buffers and scheduler and by the developer
//! tools' binary export.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** the order of [`RENDER_CLASS_CODES`] is the wire format and never changes; a kind
//! with no render class gets [`NO_CLASS`] and is never drawn or picked; a row with a non-finite
//! position is rejected, never defaulted.

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

/// One chunk instance row with its full transform, `[pid, x, y, z, rot, pitchDeg, rollDeg,
/// scale]`, as [`narrow_instance_row_v2`] narrows it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceRowV2 {
    /// Catalogue prefab id the instance places (element 0), joined against the prefab map.
    pub pid: f64,

    /// Position along world X (east), metres (element 1).
    pub x: f64,

    /// Position along world Z (north), metres (element 2).
    pub y: f64,

    /// Height (world Y), metres (element 3); 0 when absent or not finite.
    pub z: f64,

    /// Heading, degrees, turning the footprint clockwise (element 4); 0 when absent or not finite.
    pub rot: f64,

    /// Pitch about world X, degrees (element 5); 0 when absent or not finite.
    pub pitch: f64,

    /// Roll about world Z, degrees (element 6); 0 when absent or not finite.
    pub roll: f64,

    /// Uniform scale (element 7); 1 when absent, not finite or not positive.
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
#[path = "tests/render_classes_tests.rs"]
mod tests;
