//! Render-class taxonomy + instance-row narrowing — verbatim ports of `renderClassForPrefab`
//! (`worldObjectsCore.ts:48`) and `narrowInstanceRow` (`:399`). Class S depends on the exact
//! classification; the instance narrow governs which rows land in a chunk's SoA.

use serde_json::Value;

/// Render-class wire codes. **The array index IS the `VisibleSet.classes` byte**
/// (`worldObjectsCore.ts:42`) — this order is the wire format; do not reorder.
pub const RENDER_CLASS_CODES: [&str; 5] = ["building", "tree", "vegetation", "prop", "rockLarge"];

/// Unclassified sentinel (`NO_CLASS`, `worldObjectsCore.ts:268`) — never drawn or picked.
pub const NO_CLASS: u8 = 255;

/// Oversized-object half-extent threshold, meters (`OVERSIZED_HALF_EXTENT_M`, `:71`).
pub const OVERSIZED_HALF_EXTENT_M: f64 = 64.0;

/// `renderClassForPrefab(kind, cls)` (`:48`) → render-class name, or `None` (→ 255).
/// `water` draws as a building footprint only for pier/dock (T-090.5.2.2).
#[must_use]
pub fn render_class_for_prefab(kind: &str, cls: &str) -> Option<&'static str> {
    match kind {
        "building" => Some("building"),
        "water" => (cls == "pier" || cls == "dock").then_some("building"),
        "tree" => Some("tree"),
        "vegetation" => Some("vegetation"),
        "rock" => Some("rockLarge"),
        "prop" | "utility" => Some("prop"),
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

/// A JSON value that is a finite number, else `None`. Mirrors `Number.isFinite`, which does
/// **not** coerce — a string/bool/null yields `None` (row rejected or defaulted).
#[must_use]
fn finite(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite())
}

/// `narrowInstanceRow(row)` (`:399`) → `(pid, x, y, z, rot)` or reject.
///
/// Rejects a non-array or `< 3`-length row, a non-number `pid`, or a non-finite `x`/`y`.
/// `z`/`rot` default to `0.0` when absent or non-finite. `pid`/`x`/`y` keep full f64 precision.
#[must_use]
pub fn narrow_instance_row(row: &Value) -> Option<(f64, f64, f64, f64, f64)> {
    let arr = row.as_array()?;
    if arr.len() < 3 {
        return None;
    }
    // `typeof pid !== 'number'` → reject; a JSON number is always finite so no finite check here.
    let pid = arr[0].as_f64()?;
    let x = finite(arr.get(1))?;
    let y = finite(arr.get(2))?;
    let z = finite(arr.get(3)).unwrap_or(0.0);
    let rot = finite(arr.get(4)).unwrap_or(0.0);
    Some((pid, x, y, z, rot))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_class_truth_table() {
        assert_eq!(
            render_class_for_prefab("building", "residential"),
            Some("building")
        );
        assert_eq!(render_class_for_prefab("water", "pier"), Some("building"));
        assert_eq!(render_class_for_prefab("water", "dock"), Some("building"));
        assert_eq!(render_class_for_prefab("water", "buoy"), None);
        assert_eq!(render_class_for_prefab("tree", "conifer"), Some("tree"));
        assert_eq!(
            render_class_for_prefab("vegetation", "bush"),
            Some("vegetation")
        );
        assert_eq!(
            render_class_for_prefab("rock", "boulder"),
            Some("rockLarge")
        );
        assert_eq!(render_class_for_prefab("prop", "barrel"), Some("prop"));
        assert_eq!(render_class_for_prefab("utility", "pole"), Some("prop"));
        assert_eq!(render_class_for_prefab("mystery", "x"), None);
    }

    #[test]
    fn class_codes_match_wire_order() {
        assert_eq!(class_code("building"), 0);
        assert_eq!(class_code("tree"), 1);
        assert_eq!(class_code("vegetation"), 2);
        assert_eq!(class_code("prop"), 3);
        assert_eq!(class_code("rockLarge"), 4);
        assert_eq!(class_code("nope"), NO_CLASS);
    }

    #[test]
    fn narrow_instance_row_accepts_and_defaults() {
        // Full 5-tuple.
        assert_eq!(
            narrow_instance_row(&json!([9, 512.0, 700.25, 41.3, 90])),
            Some((9.0, 512.0, 700.25, 41.3, 90.0))
        );
        // Length-3 row: z/rot default to 0.
        assert_eq!(
            narrow_instance_row(&json!([3, 1.0, 2.0])),
            Some((3.0, 1.0, 2.0, 0.0, 0.0))
        );
        // Non-finite z/rot (here: strings) default to 0.
        assert_eq!(
            narrow_instance_row(&json!([3, 1.0, 2.0, "x", "y"])),
            Some((3.0, 1.0, 2.0, 0.0, 0.0))
        );
    }

    #[test]
    fn narrow_instance_row_rejects() {
        assert_eq!(narrow_instance_row(&json!("garbage")), None); // not an array
        assert_eq!(narrow_instance_row(&json!([9, 1.0])), None); // length < 3
        assert_eq!(narrow_instance_row(&json!(["x", 1.0, 2.0])), None); // pid not a number
        assert_eq!(narrow_instance_row(&json!([9, "x", 2.0])), None); // x not finite
        assert_eq!(narrow_instance_row(&json!([9, 1.0, "y"])), None); // y not finite
    }
}
