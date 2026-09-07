//! T-936.7 — the authored `tacticalGraphics[]` block: phase lines, boundaries and arrows.
//!
//! ══ Why this module exists ══════════════════════════════════════════════════════════════════
//! Every authored map graphic in the document today is a SINGLE POINT: `$defs/marker` is
//! `{x, z, icon, label, …}` and its icon enum even carries the word `phase_line`, which is a
//! one-glyph stand-in for a line the author could not actually draw. Control measures are
//! multi-point by nature — a phase line is a run of vertices, a boundary is the seam between two
//! sides, an axis of advance is a direction of attack — so the marker vocabulary cannot express
//! any of them without lying about the geometry.
//!
//! This block is `[{id, kind, points: [[x, z], …], label?, sideKey?, style?}]`. The editor writes
//! it into `meta.environment.tacticalGraphics`;
//! [`crate::mission::extensions::AUTHORED_BLOCKS`] copies it onto the compiled payload root and
//! reads it back through [`crate::mission::extensions::ExtensionBlocks`]. A mission that authors no
//! graphics still compiles to today's bytes — the carrier emits nothing when the key is absent.
//!
//! ══ T-946.53 — a row is NOT a wire ══════════════════════════════════════════════════════════
//! An `AUTHORED_BLOCKS` row alone does not reach `/compiled`, whatever `extensions.rs`'s header
//! used to claim. `flatten.rs`'s `EditorPayload` is a NAMED-FIELD struct with no
//! `#[serde(flatten)]` catch-all (deliberately — flatten would buffer every unmatched top-level
//! key of an 8 MB payload into `Content`), and `EditorPayload::authored_block_value` is a hand
//! `match` ending `_ => None`. A registered key with no field and no arm is therefore dropped
//! SILENTLY: no compile error, no diagnostic, no refusal. So this slice moved seven places at once
//! and [`tests::tactical_graphics_registered_here_must_also_be_readable_by_flatten`] plus
//! `flatten.rs`'s `authored_tactical_graphics_survive_flatten_to_mod_document` are what hold the
//! pair together for the next reader.
//!
//! ══ Point counts ═══════════════════════════════════════════════════════════════════════════
//! [`min_points`] is the per-kind floor and the perturbation target: widening the `phase_line` arm
//! to 1 turns [`tests::a_one_point_phase_line_is_refused`] red. Three of the four kinds are open
//! polylines and need two vertices; `curved_arrow` needs THREE, because a two-point Catmull-Rom
//! spline has no interior control point and degenerates to exactly the straight segment
//! `axis_of_advance` already is — an author who wants a straight arrow has a kind for it, and
//! letting `curved_arrow` collapse onto it would make the two kinds indistinguishable on the wire.
//!
//! [`MAX_POINTS`] caps the run. Unlike the count caps in `spawn_modules.rs` this is not a server
//! load argument: it is a wire argument. The graphics ride the compiled document that the game
//! server re-fetches, and an unbounded vertex list is the one field here an author could
//! accidentally make megabytes wide by dragging a draw tool.
//!
//! ══ Style ══════════════════════════════════════════════════════════════════════════════════
//! `style` reuses T-673's MARKER vocabulary rather than inventing a second one — `color` is
//! `$defs/hexColor`, `alpha` is 0..1, and `brush` is the same eight Eden fill names
//! `$defs/marker.brush` declares. `widthM` is the one addition, and it is forced by the geometry:
//! a marker is a glyph with a `size` multiplier, whereas a control measure is a STROKE and a
//! stroke needs a world width. Keeping the other three spellings identical is what lets a reader
//! that already styles markers style these with the same code.

use serde_json::{Map, Value};

/// `kind` vocabulary `$defs/tacticalGraphic.kind` declares, in schema enum order.
pub const KINDS: &[&str] = &["phase_line", "boundary", "axis_of_advance", "curved_arrow"];

/// Fill brushes `style.brush` accepts — the same eight names `$defs/marker.brush` (T-673) carries,
/// spelled identically so one reader can style a marker and a graphic with one table.
pub const BRUSHES: &[&str] = &[
    "solid",
    "border",
    "diagonal",
    "cross_diagonal",
    "horizontal",
    "vertical",
    "grid",
    "fill_diagonal",
];

/// Hard cap on `points[]` length. See the module header for why the argument is about wire size
/// rather than server load.
pub const MAX_POINTS: usize = 128;

const GRAPHIC_KEYS: &[&str] = &["id", "kind", "points", "label", "sideKey", "style"];
const STYLE_KEYS: &[&str] = &["color", "alpha", "brush", "widthM"];

/// The minimum vertex count for `kind`, or `None` when `kind` is not one of [`KINDS`].
///
/// **The T-936.7 perturbation target.** Returning `Some(1)` for `phase_line` turns
/// [`tests::a_one_point_phase_line_is_refused`] red; restore + `touch` turns it green.
#[must_use]
pub fn min_points(kind: &str) -> Option<usize> {
    match kind {
        // Open polylines: two vertices is a segment, which is the smallest honest control measure.
        "phase_line" | "boundary" | "axis_of_advance" => Some(2),
        // A Catmull-Rom spline through two points IS the straight segment `axis_of_advance`
        // already carries; three is the first count at which the kind means something different.
        "curved_arrow" => Some(3),
        _ => None,
    }
}

/// One authored tactical graphic.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredTacticalGraphic {
    /// Stable graphic id, unique across the block.
    pub id: String,
    /// One of [`KINDS`].
    pub kind: String,
    /// World-space `(x, z)` vertices, in author order. Length is in
    /// `min_points(kind)..=MAX_POINTS`.
    pub points: Vec<[f64; 2]>,
    /// Optional map label ("PL BLUE", "AXIS SABRE").
    pub label: Option<String>,
    /// Optional owning side — a `factions[].key`, not a closed set, because factions are authored.
    pub side_key: Option<String>,
    /// Optional stroke style.
    pub style: Option<TacticalGraphicStyle>,
}

/// The optional stroke style — T-673's marker vocabulary plus a world stroke width.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TacticalGraphicStyle {
    /// `#rrggbb`, the same form `$defs/hexColor` pins for markers and factions.
    pub color: Option<String>,
    /// Opacity, 0 (transparent) → 1 (opaque).
    pub alpha: Option<f64>,
    /// One of [`BRUSHES`].
    pub brush: Option<String>,
    /// Stroke width in world metres. Strictly positive.
    pub width_m: Option<f64>,
}

/// Parse an authored `tacticalGraphics[]` array.
///
/// # Errors
/// Wrong type, an empty array, a row the schema would refuse, a duplicate id, an unknown kind,
/// too few or too many points for the kind, a non-finite coordinate, a malformed `sideKey`, or a
/// style value outside its vocabulary.
pub fn parse(value: &Value) -> Result<Vec<AuthoredTacticalGraphic>, String> {
    let Some(arr) = value.as_array() else {
        return Err(format!(
            "`tacticalGraphics` must be an array, not {}",
            type_name(value)
        ));
    };
    if arr.is_empty() {
        return Err(
            "`tacticalGraphics` is empty — an empty list is omitted rather than authored".into(),
        );
    }

    let mut out = Vec::with_capacity(arr.len());
    let mut seen: Vec<String> = Vec::with_capacity(arr.len());
    for (index, item) in arr.iter().enumerate() {
        let row = parse_graphic(item, index)?;
        if seen.iter().any(|s| s == &row.id) {
            return Err(format!(
                "`tacticalGraphics[{index}]`.id is {} — each graphic id must be unique",
                quote(&row.id)
            ));
        }
        seen.push(row.id.clone());
        out.push(row);
    }
    Ok(out)
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

fn parse_graphic(value: &Value, index: usize) -> Result<AuthoredTacticalGraphic, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`tacticalGraphics[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };
    refuse_unknown(obj, GRAPHIC_KEYS, &format!("tacticalGraphics[{index}]"))?;

    let id = required_nonempty(obj, index, "id")?;
    let kind = required_nonempty(obj, index, "kind")?;
    let Some(floor) = min_points(&kind) else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.kind is {} — must be one of {}",
            quote(&kind),
            KINDS.join(", ")
        ));
    };

    let points = parse_points(obj, index, &kind, floor)?;
    let label = optional_nonempty(obj, index, "label")?;
    let side_key = parse_side_key(obj, index)?;
    let style = parse_style(obj, index)?;

    Ok(AuthoredTacticalGraphic {
        id,
        kind,
        points,
        label,
        side_key,
        style,
    })
}

fn parse_points(
    obj: &Map<String, Value>,
    index: usize,
    kind: &str,
    floor: usize,
) -> Result<Vec<[f64; 2]>, String> {
    let Some(raw) = obj.get("points") else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.points is required and is missing"
        ));
    };
    let Some(arr) = raw.as_array() else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.points must be an array, not {}",
            type_name(raw)
        ));
    };
    if arr.len() < floor {
        return Err(format!(
            "`tacticalGraphics[{index}]`.points has {} point(s) — a {kind} needs at least {floor}",
            arr.len()
        ));
    }
    if arr.len() > MAX_POINTS {
        return Err(format!(
            "`tacticalGraphics[{index}]`.points has {} points — at most {MAX_POINTS} are carried",
            arr.len()
        ));
    }

    let mut out = Vec::with_capacity(arr.len());
    for (vertex, item) in arr.iter().enumerate() {
        let Some(pair) = item.as_array() else {
            return Err(format!(
                "`tacticalGraphics[{index}]`.points[{vertex}] must be an [x, z] pair, not {}",
                type_name(item)
            ));
        };
        if pair.len() != 2 {
            return Err(format!(
                "`tacticalGraphics[{index}]`.points[{vertex}] has {} value(s) — a vertex is exactly [x, z]",
                pair.len()
            ));
        }
        let x = finite_coord(&pair[0], index, vertex, "x")?;
        let z = finite_coord(&pair[1], index, vertex, "z")?;
        out.push([x, z]);
    }
    Ok(out)
}

fn finite_coord(raw: &Value, index: usize, vertex: usize, axis: &str) -> Result<f64, String> {
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.points[{vertex}].{axis} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() {
        return Err(format!(
            "`tacticalGraphics[{index}]`.points[{vertex}].{axis} is {n} — must be a finite number"
        ));
    }
    Ok(n)
}

fn parse_side_key(obj: &Map<String, Value>, index: usize) -> Result<Option<String>, String> {
    let Some(key) = optional_nonempty(obj, index, "sideKey")? else {
        return Ok(None);
    };
    // `$defs/factionKey` is `^[a-z][a-z0-9_]*$` — a PATTERN and not an enum, because
    // `factions[].key` is authored. Checking the shape is all this layer honestly can do; a
    // reference to a faction that does not exist is a cross-block question the document validator
    // owns, not this block's.
    let mut chars = key.chars();
    let head_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase());
    let tail_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !head_ok || !tail_ok {
        return Err(format!(
            "`tacticalGraphics[{index}]`.sideKey is {} — a faction key is lowercase \
             ^[a-z][a-z0-9_]*$",
            quote(&key)
        ));
    }
    Ok(Some(key))
}

fn parse_style(
    obj: &Map<String, Value>,
    index: usize,
) -> Result<Option<TacticalGraphicStyle>, String> {
    let Some(raw) = obj.get("style") else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let Some(style) = raw.as_object() else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.style must be an object, not {}",
            type_name(raw)
        ));
    };
    refuse_unknown(
        style,
        STYLE_KEYS,
        &format!("tacticalGraphics[{index}].style"),
    )?;

    let color = match style.get("color") {
        None => None,
        Some(v) => {
            let Some(s) = v.as_str() else {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.color must be a string, not {}",
                    type_name(v)
                ));
            };
            if !is_hex_color(s) {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.color is {} — must be #rrggbb",
                    quote(s)
                ));
            }
            Some(s.to_string())
        }
    };

    let alpha = match style.get("alpha") {
        None => None,
        Some(v) => {
            let Some(n) = v.as_f64() else {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.alpha must be a number, not {}",
                    type_name(v)
                ));
            };
            if !n.is_finite() || !(0.0..=1.0).contains(&n) {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.alpha is {n} — must be between 0 and 1"
                ));
            }
            Some(n)
        }
    };

    let brush = match style.get("brush") {
        None => None,
        Some(v) => {
            let Some(s) = v.as_str() else {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.brush must be a string, not {}",
                    type_name(v)
                ));
            };
            if !BRUSHES.contains(&s) {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.brush is {} — must be one of {}",
                    quote(s),
                    BRUSHES.join(", ")
                ));
            }
            Some(s.to_string())
        }
    };

    let width_m = match style.get("widthM") {
        None => None,
        Some(v) => {
            let Some(n) = v.as_f64() else {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.widthM must be a number, not {}",
                    type_name(v)
                ));
            };
            if !n.is_finite() || n <= 0.0 {
                return Err(format!(
                    "`tacticalGraphics[{index}]`.style.widthM is {n} — must be above zero"
                ));
            }
            Some(n)
        }
    };

    Ok(Some(TacticalGraphicStyle {
        color,
        alpha,
        brush,
        width_m,
    }))
}

fn is_hex_color(s: &str) -> bool {
    let Some(body) = s.strip_prefix('#') else {
        return false;
    };
    body.len() == 6 && body.chars().all(|c| c.is_ascii_hexdigit())
}

fn refuse_unknown(obj: &Map<String, Value>, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "`{path}` carries {key:?}, which the schema does not declare \
                 (additionalProperties is false)"
            ));
        }
    }
    Ok(())
}

fn required_nonempty(obj: &Map<String, Value>, index: usize, key: &str) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.{key} is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`tacticalGraphics[{index}]`.{key} is blank"));
    }
    Ok(s.to_string())
}

fn optional_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`tacticalGraphics[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`tacticalGraphics[{index}]`.{key} is blank"));
    }
    Ok(Some(s.to_string()))
}

fn quote(s: &str) -> String {
    format!("{s:?}")
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::compile::compile_payload;
    use crate::mission::extensions::{
        AUTHORED_BLOCKS, DOCUMENT_OWNED_BLOCKS, ExtensionBlocks, copy_authored_blocks,
        is_authored_block,
    };
    use serde_json::json;

    fn phase_line() -> Value {
        json!({
            "id": "tg-phase",
            "kind": "phase_line",
            "points": [[1000.0, 2000.0], [1400.0, 2100.0]],
            "label": "PL BLUE",
            "sideKey": "blufor",
            "style": {"color": "#3388ff", "alpha": 0.8}
        })
    }

    fn boundary() -> Value {
        json!({
            "id": "tg-bound",
            "kind": "boundary",
            "points": [[900.0, 1800.0], [1200.0, 1900.0], [1500.0, 2400.0]]
        })
    }

    fn axis_of_advance() -> Value {
        json!({
            "id": "tg-axis",
            "kind": "axis_of_advance",
            "points": [[800.0, 1200.0], [1600.0, 2600.0]],
            "label": "AXIS SABRE"
        })
    }

    fn curved_arrow() -> Value {
        json!({
            "id": "tg-arrow",
            "kind": "curved_arrow",
            "points": [[700.0, 1100.0], [1100.0, 1500.0], [1700.0, 1400.0]],
            "style": {"brush": "solid", "color": "#ff2222", "widthM": 24.0}
        })
    }

    fn one_of_each() -> Value {
        json!([phase_line(), boundary(), axis_of_advance(), curved_arrow()])
    }

    fn compile_env_with_graphics(block: &Value) -> Value {
        compile_payload(
            &json!({
                "meta": {
                    "terrain": "everon",
                    "environment": { "weather": "clear", "tacticalGraphics": block }
                }
            })
            .to_string(),
            "{}",
            false,
        )
    }

    #[test]
    fn one_graphic_of_each_kind_parses() {
        let got = parse(&one_of_each()).expect("parses");
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].kind, "phase_line");
        assert_eq!(got[0].points, vec![[1000.0, 2000.0], [1400.0, 2100.0]]);
        assert_eq!(got[0].label.as_deref(), Some("PL BLUE"));
        assert_eq!(got[0].side_key.as_deref(), Some("blufor"));
        assert_eq!(
            got[0].style.as_ref().and_then(|s| s.color.as_deref()),
            Some("#3388ff")
        );
        assert_eq!(got[0].style.as_ref().and_then(|s| s.alpha), Some(0.8));
        assert_eq!(got[1].kind, "boundary");
        assert!(got[1].label.is_none());
        assert!(got[1].style.is_none());
        assert_eq!(got[2].kind, "axis_of_advance");
        assert_eq!(got[3].kind, "curved_arrow");
        assert_eq!(got[3].points.len(), 3);
        assert_eq!(
            got[3].style.as_ref().and_then(|s| s.brush.as_deref()),
            Some("solid")
        );
        assert_eq!(got[3].style.as_ref().and_then(|s| s.width_m), Some(24.0));
    }

    /// **The T-936.7 perturbation.** Widen [`min_points`]'s `phase_line` arm to 1 and this goes
    /// red — the predicate is what the refusal rests on, not a literal buried in `parse_points`.
    #[test]
    fn a_one_point_phase_line_is_refused() {
        // The REFUSAL first, so a widened floor reds this on the behaviour rather than on the
        // vocabulary table below it.
        let err = parse(&json!([{
            "id": "tg-short",
            "kind": "phase_line",
            "points": [[1.0, 2.0]]
        }]))
        .expect_err("a one-point phase line is not a line");
        assert!(err.contains("at least 2"), "{err}");

        assert_eq!(min_points("phase_line"), Some(2));
        assert_eq!(min_points("boundary"), Some(2));
        assert_eq!(min_points("axis_of_advance"), Some(2));
        assert_eq!(min_points("curved_arrow"), Some(3));
        assert_eq!(min_points("marker"), None);
    }

    /// A `curved_arrow` needs a third vertex or it IS an `axis_of_advance` — the module header's
    /// distinctness argument, asserted rather than left as prose.
    #[test]
    fn a_two_point_curved_arrow_is_refused_but_a_two_point_axis_is_not() {
        let err = parse(&json!([{
            "id": "tg-flat",
            "kind": "curved_arrow",
            "points": [[1.0, 2.0], [3.0, 4.0]]
        }]))
        .expect_err("two points cannot curve");
        assert!(err.contains("at least 3"), "{err}");

        parse(&json!([{
            "id": "tg-axis",
            "kind": "axis_of_advance",
            "points": [[1.0, 2.0], [3.0, 4.0]]
        }]))
        .expect("the straight kind accepts two");
    }

    #[test]
    fn a_non_finite_coordinate_is_refused() {
        // `serde_json` cannot even hold a NaN, so the wire form an author can actually produce is
        // a string or a null in the coordinate slot — both of which must be refused by TYPE, and
        // the finite check guards the f64 path a hand-built `Value` can still reach.
        let err = parse(&json!([{
            "id": "tg-nan",
            "kind": "phase_line",
            "points": [[1.0, 2.0], ["3.0", 4.0]]
        }]))
        .expect_err("a string is not a coordinate");
        assert!(err.contains("must be a number"), "{err}");

        let mut graphic = json!({
            "id": "tg-inf",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [0.0, 0.0]]
        });
        graphic["points"][1][0] = Value::from(f64::MAX);
        // f64::MAX is finite and therefore legal; the guard is about NaN/inf, which serde_json
        // serialises as null and which the type arm above already refuses.
        parse(&json!([graphic])).expect("a large finite coordinate is legal");
    }

    #[test]
    fn a_vertex_that_is_not_a_pair_is_refused() {
        let err = parse(&json!([{
            "id": "tg-triple",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [3.0, 4.0, 5.0]]
        }]))
        .expect_err("three values is not [x, z]");
        assert!(err.contains("exactly [x, z]"), "{err}");
    }

    #[test]
    fn an_unknown_kind_and_an_unknown_key_are_refused() {
        let err = parse(&json!([{
            "id": "tg-x",
            "kind": "fire_support_line",
            "points": [[1.0, 2.0], [3.0, 4.0]]
        }]))
        .expect_err("unknown kind");
        assert!(err.contains("must be one of"), "{err}");

        let err = parse(&json!([{
            "id": "tg-x",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [3.0, 4.0]],
            "colour": "#ffffff"
        }]))
        .expect_err("unknown key");
        assert!(err.contains("additionalProperties"), "{err}");
    }

    #[test]
    fn a_style_outside_the_marker_vocabulary_is_refused() {
        let bad = |style: Value| {
            parse(&json!([{
                "id": "tg-s",
                "kind": "phase_line",
                "points": [[1.0, 2.0], [3.0, 4.0]],
                "style": style
            }]))
            .expect_err("bad style")
        };
        assert!(bad(json!({"color": "3388ff"})).contains("#rrggbb"));
        assert!(bad(json!({"color": "#3388f"})).contains("#rrggbb"));
        assert!(bad(json!({"alpha": 1.5})).contains("between 0 and 1"));
        assert!(bad(json!({"alpha": -0.1})).contains("between 0 and 1"));
        assert!(bad(json!({"brush": "stipple"})).contains("must be one of"));
        assert!(bad(json!({"widthM": 0.0})).contains("above zero"));
        assert!(bad(json!({"dash": true})).contains("additionalProperties"));

        // The eight brushes are exactly `$defs/marker.brush`'s, and every one is accepted.
        for brush in BRUSHES {
            parse(&json!([{
                "id": "tg-s",
                "kind": "phase_line",
                "points": [[1.0, 2.0], [3.0, 4.0]],
                "style": {"brush": brush}
            }]))
            .unwrap_or_else(|e| panic!("{brush} must be legal: {e}"));
        }
    }

    #[test]
    fn a_malformed_side_key_is_refused_and_an_authored_one_is_not() {
        let err = parse(&json!([{
            "id": "tg-s",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [3.0, 4.0]],
            "sideKey": "BLUFOR"
        }]))
        .expect_err("uppercase is not a faction key");
        assert!(err.contains("lowercase"), "{err}");

        // `factions[].key` is AUTHORED, so the check is the pattern and not a closed set.
        let got = parse(&json!([{
            "id": "tg-s",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [3.0, 4.0]],
            "sideKey": "task_force_7"
        }]))
        .expect("an authored faction key passes");
        assert_eq!(got[0].side_key.as_deref(), Some("task_force_7"));
    }

    #[test]
    fn duplicate_ids_and_empty_and_oversized_blocks_are_refused() {
        let err = parse(&json!([phase_line(), phase_line()])).expect_err("dup");
        assert!(err.contains("unique"), "{err}");

        let err = parse(&json!([])).expect_err("empty");
        assert!(err.contains("empty"), "{err}");

        let err = parse(&json!({})).expect_err("not an array");
        assert!(err.contains("must be an array"), "{err}");

        let too_many: Vec<Value> = (0..=MAX_POINTS).map(|i| json!([i as f64, 0.0])).collect();
        let err = parse(&json!([{
            "id": "tg-long",
            "kind": "phase_line",
            "points": too_many
        }]))
        .expect_err("over the cap");
        assert!(err.contains("at most 128"), "{err}");
    }

    #[test]
    fn tactical_graphics_is_registered_on_the_carrier() {
        assert!(
            is_authored_block("tacticalGraphics"),
            "T-936.7's row must be in AUTHORED_BLOCKS or the carrier never emits it"
        );
        assert!(
            !DOCUMENT_OWNED_BLOCKS.contains(&"tacticalGraphics"),
            "tacticalGraphics is optional — it rides ExtensionBlocks"
        );
        assert_eq!(
            KINDS,
            ["phase_line", "boundary", "axis_of_advance", "curved_arrow"]
        );
        assert_eq!(MAX_POINTS, 128);
    }

    /// **T-946.53, stated where the next reader will be standing.** A row in `AUTHORED_BLOCKS` is
    /// only half the passage: `flatten.rs` reads it back through a hand `match` that ends
    /// `_ => None`, so a registered key with no arm is dropped in silence. This asserts the pair
    /// through the real reader rather than trusting the row.
    #[test]
    fn tactical_graphics_registered_here_must_also_be_readable_by_flatten() {
        assert_eq!(
            AUTHORED_BLOCKS.len(),
            7,
            "the seven T-936 blocks; a row added without a flatten field is a silent drop"
        );
        let p = compile_env_with_graphics(&one_of_each());
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert_eq!(carried.get("tacticalGraphics"), Some(&one_of_each()));
    }

    #[test]
    fn a_mission_with_one_of_each_kind_copies_to_the_payload_root() {
        let block = one_of_each();
        let p = compile_env_with_graphics(&block);
        assert_eq!(
            p["tacticalGraphics"], block,
            "AUTHORED_BLOCKS must promote tacticalGraphics out of the env bag: {p:#}"
        );
        assert_eq!(p["tacticalGraphics"].as_array().expect("array").len(), 4);
        assert_eq!(
            p["environment"]["tacticalGraphics"], block,
            "the bag reaches the wire unchanged — that is the reload path"
        );
    }

    #[test]
    fn an_unauthored_payload_still_omits_the_tactical_graphics_key() {
        let p = compile_payload(
            &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}})
                .to_string(),
            "{}",
            false,
        );
        assert!(
            p.get("tacticalGraphics").is_none(),
            "parity: no graphics authored ⇒ no tacticalGraphics key: {p:#}"
        );
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(carried.get("tacticalGraphics").is_none());
    }

    /// The unlisted-key witness, re-pointed one last time. `tacticalGraphics` is the SEVENTH and
    /// final T-936 block, so there is no eighth key to hand the baton to — the witness now uses a
    /// name that is not a block and never will be, which is what it always meant.
    #[test]
    fn an_unlisted_environment_key_is_not_promoted() {
        let env = json!({"weather": "clear", "notAnAuthoredBlock": []});
        let mut dst = Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);
        assert!(!copied.contains(&"notAnAuthoredBlock"), "{copied:?}");
        assert!(
            !dst.contains_key("notAnAuthoredBlock"),
            "an unlisted key stays parked: {dst:?}"
        );
    }

    /// A MALFORMED block is refused with a sentence rather than carried to a schema failure — the
    /// `AUTHORED_BLOCKS` contract every row owes, driven through the real carrier.
    #[test]
    fn a_malformed_block_is_refused_at_the_carrier_with_a_readable_clause() {
        let p = compile_env_with_graphics(&json!([{
            "id": "tg-short",
            "kind": "phase_line",
            "points": [[1.0, 2.0]]
        }]));
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(carried.get("tacticalGraphics").is_none());
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert_eq!(refusals[0].0, "tacticalGraphics");
        assert!(refusals[0].1.contains("at least 2"), "{:?}", refusals[0]);
    }
}
