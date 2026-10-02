//! Role: shapes.
//! Position: `mission/extensions/tactical_graphics` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Value, quote, type_name};

/// `kind` vocabulary `$defs/tacticalGraphic.kind` declares, in schema enum order.
pub const KINDS: &[&str] = &["phase_line", "boundary", "axis_of_advance", "curved_arrow"];

/// Canonical brushes value.
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

/// Hard cap on `points[]` length. See the module header for why the argument is about wire size rather than server load.
pub const MAX_POINTS: usize = 128;

/// Canonical graphic keys value.
pub(super) const GRAPHIC_KEYS: &[&str] = &["id", "kind", "points", "label", "sideKey", "style"];

/// Canonical style keys value.
pub(super) const STYLE_KEYS: &[&str] = &["color", "alpha", "brush", "widthM"];

/// The minimum vertex count for `kind`, or `None` when `kind` is not one of [`KINDS`].
#[must_use]
pub fn min_points(kind: &str) -> Option<usize> {
    match kind {
        "phase_line" | "boundary" | "axis_of_advance" => Some(2),

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

    /// World-space `(x, z)` vertices, in author order. Length is in `min_points(kind)..=MAX_POINTS`.
    pub points: Vec<[f64; 2]>,

    /// Optional map label ("PL BLUE", "AXIS SABRE").
    pub label: Option<String>,

    /// Optional owning side — a `factions[].key`, not a closed set, because factions are authored.
    pub side_key: Option<String>,

    /// Optional stroke style.
    pub style: Option<TacticalGraphicStyle>,
}

/// Domain representation of tactical graphic style.
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

/// [`parse`] with the value discarded — the [`crate::data::scenario::extensions::AUTHORED_BLOCKS`] row's validator.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

/// Parse graphic using the supplied domain data.
pub(super) fn parse_graphic(
    value: &Value,
    index: usize,
) -> Result<AuthoredTacticalGraphic, String> {
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

/// Parse points using the supplied domain data.
pub(super) fn parse_points(
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

/// Finite coord using the supplied domain data.
pub(super) fn finite_coord(
    raw: &Value,
    index: usize,
    vertex: usize,
    axis: &str,
) -> Result<f64, String> {
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

/// Parse side key using the supplied domain data.
pub(super) fn parse_side_key(
    obj: &Map<String, Value>,
    index: usize,
) -> Result<Option<String>, String> {
    let Some(key) = optional_nonempty(obj, index, "sideKey")? else {
        return Ok(None);
    };

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

/// Parse style using the supplied domain data.
pub(super) fn parse_style(
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

/// Is hex color using the supplied domain data.
pub(super) fn is_hex_color(s: &str) -> bool {
    let Some(body) = s.strip_prefix('#') else {
        return false;
    };
    body.len() == 6 && body.chars().all(|c| c.is_ascii_hexdigit())
}

/// Refuse unknown using the supplied domain data.
pub(super) fn refuse_unknown(
    obj: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), String> {
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

/// Required nonempty using the supplied domain data.
pub(super) fn required_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<String, String> {
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

/// Optional nonempty using the supplied domain data.
pub(super) fn optional_nonempty(
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
