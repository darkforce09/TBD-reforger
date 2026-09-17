//! Tactical graphic rows, geometry, and picking.
use serde_json::Value;

#[cfg(any(test, target_arch = "wasm32"))]
pub use website_map_engine::data::store::operations::tactical_graphics::TacticalDraft;

/// Screen pixel tolerance for selecting a tactical graphic.
pub(crate) const TG_PICK_PX: f64 = 6.0;

/// Screen pixel tolerance for selecting a graphic vertex.
pub(crate) const TG_VERTEX_PICK_PX: f64 = 9.0;

/// Samples used to approximate one curve span.
pub(crate) const CURVE_SAMPLES_PER_SPAN: usize = 12;

const ARROW_HEAD_FRAC: f64 = 0.08;
const ARROW_HEAD_MIN_M: f64 = 25.0;
const ARROW_HEAD_MAX_M: f64 = 400.0;
const ARROW_HEAD_ANGLE: f64 = 0.5;

const BOUNDARY_TICK_M: f64 = 40.0;

/// Highlight color used for a selected tactical graphic.
pub(crate) const TG_SELECTED_RGBA: [f32; 4] = [1.0, 0.78, 0.30, 1.0];

/// Returns the default color assigned to a graphic kind.
#[must_use]
pub(crate) fn default_kind_rgba(kind: &str) -> [f32; 4] {
    match kind {
        "phase_line" => [0.68, 0.78, 1.0, 0.90],
        "boundary" => [0.96, 0.88, 0.37, 0.90],
        "axis_of_advance" => [0.55, 0.90, 0.55, 0.90],
        "curved_arrow" => [0.95, 0.35, 0.30, 0.90],
        _ => [0.85, 0.85, 0.85, 0.85],
    }
}

/// A materialized tactical graphic ready for drawing and picking.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TacticalGraphic {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) points: Vec<[f64; 2]>,
    pub(crate) label: String,
    pub(crate) rgba: [f32; 4],
}

/// Parses tactical graphics from mission environment data.
#[must_use]
pub(crate) fn tactical_graphics_from_env(env: &Value) -> Vec<TacticalGraphic> {
    let Some(arr) = env.get("tacticalGraphics").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("");
        let kind = row.get("kind").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() || kind.is_empty() {
            continue;
        }
        let mut points = Vec::new();
        if let Some(pts) = row.get("points").and_then(Value::as_array) {
            for p in pts {
                let Some(pair) = p.as_array() else { continue };
                let (Some(x), Some(z)) = (
                    pair.first().and_then(Value::as_f64),
                    pair.get(1).and_then(Value::as_f64),
                ) else {
                    continue;
                };
                if x.is_finite() && z.is_finite() {
                    points.push([x, z]);
                }
            }
        }
        if points.len() < 2 {
            continue;
        }
        out.push(TacticalGraphic {
            id: id.to_string(),
            kind: kind.to_string(),
            points,
            label: row
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            rgba: resolve_rgba(kind, row.get("style")),
        });
    }
    out
}

fn resolve_rgba(kind: &str, style: Option<&Value>) -> [f32; 4] {
    let mut rgba = default_kind_rgba(kind);
    let Some(style) = style else { return rgba };
    if let Some(hex) = style.get("color").and_then(Value::as_str) {
        if let Some(rgb) = hex_to_rgb(hex) {
            rgba[0] = rgb[0];
            rgba[1] = rgb[1];
            rgba[2] = rgb[2];
        }
    }
    if let Some(a) = style.get("alpha").and_then(Value::as_f64) {
        if a.is_finite() && (0.0..=1.0).contains(&a) {
            #[allow(clippy::cast_possible_truncation)]
            {
                rgba[3] = a as f32;
            }
        }
    }
    rgba
}

#[must_use]
fn hex_to_rgb(hex: &str) -> Option<[f32; 3]> {
    let body = hex.strip_prefix('#')?;
    if body.len() != 6 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let c = |i: usize| -> f32 {
        f32::from(u8::from_str_radix(&body[i..i + 2], 16).unwrap_or(0)) / 255.0
    };
    Some([c(0), c(2), c(4)])
}

/// Samples a smooth polyline through control points.
#[must_use]
pub(crate) fn catmull_rom(points: &[[f64; 2]], per_span: usize) -> Vec<[f64; 2]> {
    if points.len() < 3 || per_span == 0 {
        return points.to_vec();
    }
    let n = points.len();
    let at = |i: isize| -> [f64; 2] {
        let idx = i.clamp(0, (n - 1) as isize);
        #[allow(clippy::cast_sign_loss)]
        points[idx as usize]
    };

    let mut out: Vec<[f64; 2]> = Vec::with_capacity((n - 1) * per_span + 1);
    out.push(points[0]);
    for span in 0..(n - 1) {
        let i = span as isize;
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let knot = |t: f64, a: [f64; 2], b: [f64; 2]| -> f64 {
            let d = (b[0] - a[0]).hypot(b[1] - a[1]);
            t + d.sqrt().max(1e-6)
        };
        let t0 = 0.0;
        let t1 = knot(t0, p0, p1);
        let t2 = knot(t1, p1, p2);
        let t3 = knot(t2, p2, p3);

        for s in 1..=per_span {
            let u = s as f64 / per_span as f64;
            let t = t1 + u * (t2 - t1);
            out.push(catmull_rom_point(p0, p1, p2, p3, t0, t1, t2, t3, t));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn catmull_rom_point(
    p0: [f64; 2],
    p1: [f64; 2],
    p2: [f64; 2],
    p3: [f64; 2],
    t0: f64,
    t1: f64,
    t2: f64,
    t3: f64,
    t: f64,
) -> [f64; 2] {
    let lerp = |a: [f64; 2], b: [f64; 2], ta: f64, tb: f64| -> [f64; 2] {
        let d = tb - ta;
        if d.abs() < f64::EPSILON {
            return a;
        }
        let w = (tb - t) / d;
        let v = (t - ta) / d;
        [w.mul_add(a[0], v * b[0]), w.mul_add(a[1], v * b[1])]
    };
    let a1 = lerp(p0, p1, t0, t1);
    let a2 = lerp(p1, p2, t1, t2);
    let a3 = lerp(p2, p3, t2, t3);
    let b1 = lerp(a1, a2, t0, t2);
    let b2 = lerp(a2, a3, t1, t3);
    lerp(b1, b2, t1, t2)
}

/// Returns the drawable points of a graphic.
#[must_use]
pub(crate) fn drawn_polyline(g: &TacticalGraphic) -> Vec<[f64; 2]> {
    if g.kind == "curved_arrow" {
        catmull_rom(&g.points, CURVE_SAMPLES_PER_SPAN)
    } else {
        g.points.clone()
    }
}

/// Reports whether a graphic kind draws an arrowhead.
#[must_use]
pub(crate) fn kind_has_arrowhead(kind: &str) -> bool {
    matches!(kind, "axis_of_advance" | "curved_arrow")
}

#[must_use]
fn arrowhead_segments(poly: &[[f64; 2]]) -> Vec<([f64; 2], [f64; 2])> {
    if poly.len() < 2 {
        return Vec::new();
    }
    let tip = poly[poly.len() - 1];
    let prev = poly[poly.len() - 2];
    let (dx, dy) = (tip[0] - prev[0], tip[1] - prev[1]);
    let span = dx.hypot(dy);
    if span <= f64::EPSILON {
        return Vec::new();
    }
    let total: f64 = poly
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
        .sum();
    let len = (total * ARROW_HEAD_FRAC).clamp(ARROW_HEAD_MIN_M, ARROW_HEAD_MAX_M);
    let (ux, uy) = (-dx / span, -dy / span);
    let (c, s) = (ARROW_HEAD_ANGLE.cos(), ARROW_HEAD_ANGLE.sin());
    let barb = |sign: f64| -> [f64; 2] {
        let rx = ux.mul_add(c, -(uy * s * sign));
        let ry = (ux * s).mul_add(sign, uy * c);
        [rx.mul_add(len, tip[0]), ry.mul_add(len, tip[1])]
    };
    vec![(tip, barb(1.0)), (tip, barb(-1.0))]
}

#[must_use]
fn boundary_ticks(points: &[[f64; 2]]) -> Vec<([f64; 2], [f64; 2])> {
    let mut out = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        let (a, b) = if i + 1 < points.len() {
            (*p, points[i + 1])
        } else {
            (points[i - 1], *p)
        };
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        if len <= f64::EPSILON {
            continue;
        }
        let (nx, ny) = (-dy / len, dx / len);
        out.push((
            [
                nx.mul_add(BOUNDARY_TICK_M, p[0]),
                ny.mul_add(BOUNDARY_TICK_M, p[1]),
            ],
            [
                nx.mul_add(-BOUNDARY_TICK_M, p[0]),
                ny.mul_add(-BOUNDARY_TICK_M, p[1]),
            ],
        ));
    }
    out
}

/// Builds all drawable line segments of a graphic.
#[must_use]
pub(crate) fn graphic_segments(g: &TacticalGraphic) -> Vec<([f64; 2], [f64; 2])> {
    let poly = drawn_polyline(g);
    let mut segs: Vec<([f64; 2], [f64; 2])> = poly.windows(2).map(|w| (w[0], w[1])).collect();
    if kind_has_arrowhead(&g.kind) {
        segs.extend(arrowhead_segments(&poly));
    }
    if g.kind == "boundary" {
        segs.extend(boundary_ticks(&g.points));
    }
    segs
}

/// Builds GPU lane vertices for tactical graphics.
#[must_use]
pub(crate) fn tactical_lane_verts(
    graphics: &[TacticalGraphic],
    selected: Option<&str>,
) -> Vec<f32> {
    let mut v = Vec::new();
    for g in graphics {
        let c = if selected.is_some_and(|s| s == g.id) {
            TG_SELECTED_RGBA
        } else {
            g.rgba
        };
        #[allow(clippy::cast_possible_truncation)]
        for (a, b) in graphic_segments(g) {
            for p in [a, b] {
                v.push(p[0] as f32);
                v.push(p[1] as f32);
                v.extend_from_slice(&c);
            }
        }
    }
    v
}

/// Counts line segments in a tactical lane vertex array.
#[must_use]
pub(crate) fn lane_segment_count(verts: &[f32]) -> u32 {
    #[allow(clippy::cast_possible_truncation)]
    {
        (verts.len() / 12) as u32
    }
}

fn dist_to_segment(px: f64, py: f64, a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx.mul_add(dx, dy * dy);
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((px - a[0]) * dx + (py - a[1]) * dy) / len2).clamp(0.0, 1.0)
    };
    (px - t.mul_add(dx, a[0])).hypot(py - t.mul_add(dy, a[1]))
}

/// Finds the nearest selectable graphic under a screen point.
#[must_use]
pub(crate) fn pick_tactical_graphic(
    graphics: &[TacticalGraphic],
    wx: f64,
    wy: f64,
    tol_m: f64,
) -> Option<String> {
    let mut best: Option<(f64, &str)> = None;
    for g in graphics {
        for (a, b) in graphic_segments(g) {
            let d = dist_to_segment(wx, wy, a, b);
            if d <= tol_m && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, g.id.as_str()));
            }
        }
    }
    best.map(|(_, id)| id.to_string())
}

/// Finds a selectable graphic vertex under a screen point.
#[must_use]
pub(crate) fn pick_tactical_vertex(
    graphics: &[TacticalGraphic],
    wx: f64,
    wy: f64,
    tol_m: f64,
) -> Option<(String, usize)> {
    let mut best: Option<(f64, &str, usize)> = None;
    for g in graphics {
        for (i, p) in g.points.iter().enumerate() {
            let d = (wx - p[0]).hypot(wy - p[1]);
            if d <= tol_m && best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, g.id.as_str(), i));
            }
        }
    }
    best.map(|(_, id, i)| (id.to_string(), i))
}

/// Reads materialized tactical graphics from the live mission document.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub(crate) fn live_tactical_graphics(
    core: &website_map_engine::data::store::MissionDocCore,
) -> Vec<TacticalGraphic> {
    let Ok(root) = serde_json::from_str::<Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    root.get("meta")
        .and_then(|m| m.get("environment"))
        .map_or_else(Vec::new, tactical_graphics_from_env)
}

#[cfg(test)]
#[path = "tests/tactical_graphics/geometry_and_style.rs"]
mod tests;
