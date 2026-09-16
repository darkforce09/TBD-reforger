//! Role: parse the briefing's marker rows once into the four row-aligned columns the marker lane
//! uploads.
//! Position: `editing::lanes` in the map engine.
//! Signals & state: none; the parse is a pure map from the briefing marker rows JSON to four
//! parallel arrays.
//! Invariants: one pass over one list, so row *i* is the same marker in all four columns; the
//! authored icon travels as its ALIAS and is mapped to a glyph by the renderer's single alias
//! table, never here.

/// The four parallel marker-lane arrays, parsed ONCE from `briefing_marker_rows_json` (the sole
/// schema-legal marker surface, whose rows carry `x` / `z` / `factionId` / `icon` / `label`):
///
///   * `xy` — interleaved world `[x, z, …]` in metres,
///   * `tints` — packed RGBA8 side tint per marker, from the row's `factionId`,
///   * `icons` — each marker's authored `icon` ALIAS, carried verbatim,
///   * `captions` — each marker's `label`, the caption shown on the map.
///
/// The alias-to-glyph MAPPING is deliberately not done here: the renderer owns one alias table and
/// carrying the alias string keeps that mapping in its single home.
///
/// Malformed or empty input yields four empty arrays rather than a panic — the feed's early
/// returns rely on it, and a half-written document is exactly what a restore can present.
#[must_use]
pub fn marker_lane_fields(marker_rows_json: &str) -> (Vec<f32>, Vec<u8>, Vec<String>, Vec<String>) {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(marker_rows_json) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };
    let Some(arr) = rows.as_array() else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };
    let mut xy = Vec::with_capacity(arr.len() * 2);
    let mut tints = Vec::with_capacity(arr.len() * 4);
    let mut icons = Vec::with_capacity(arr.len());
    let mut captions = Vec::with_capacity(arr.len());
    for r in arr {
        let num = |k: &str| r.get(k).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        #[allow(clippy::cast_possible_truncation)]
        {
            xy.push(num("x") as f32);
            xy.push(num("z") as f32);
        }
        let faction = r
            .get("factionId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let side = faction.strip_prefix("faction-").unwrap_or(faction);
        tints.extend_from_slice(&crate::overlay::symbology::roles::classify::side_rgba(side));
        let str_field = |k: &str| {
            r.get(k)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        icons.push(str_field("icon"));
        captions.push(str_field("label"));
    }
    (xy, tints, icons, captions)
}
