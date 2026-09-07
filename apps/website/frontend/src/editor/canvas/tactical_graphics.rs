//! T-936.7 — the canvas half of `tacticalGraphics[]`: draw, pick and drag the four control
//! measures.
//!
//! ══ What this module is ═════════════════════════════════════════════════════════════════════
//! The pure belt for the tactical-graphics lane, in the exact shape T-780 gave the connection
//! line and T-784 gave the comment glyph: **one document read, drawn AND picked**.
//! [`live_tactical_graphics`] is the read, [`tactical_lane_verts`] PACKS that read for the GPU,
//! [`pick_tactical_graphic`] and [`pick_tactical_vertex`] HIT-TEST the same `Vec`. What is drawn
//! and what a click can find are one set by construction rather than two parsers kept in step by
//! hope — which is the defect T-784 recorded when the comment glyph had a private second parse.
//!
//! Everything here except [`live_tactical_graphics`] is pure and native-testable, which is the
//! whole reason the parse/pack/pick trio lives in this file rather than in the wasm-only gesture
//! and history modules that call it.
//!
//! ══ NO NEW RENDER LANE ══════════════════════════════════════════════════════════════════════
//! The graphics ride `LaneRole::MissionZones` (`role_id::MISSION_ZONES`) through the generic
//! `RenderEngine::upload_hairline_segments`, the same typed-free path `SquadLinks` uses. Nothing
//! in `map-engine-render` changes — no `LaneRole` variant, no `role_id`, no pipeline, no atlas.
//!
//! **That lane is declared for zone rings and is squatted here, deliberately and visibly.**
//! T-592 declared `MissionZones` as "mission zone rings … one flat `[x,y,r,g,b,a]…` LineList",
//! but NOTHING in the editor uploads to it: the zone ring producer was never written, and the
//! only other user in the tree (`pages/debug/building_interior.rs`) already reuses this same role
//! for an LoS ray — so reuse is the established pattern here, not an invention. A tactical
//! graphic is mission line geometry in exactly this draw position (above `Grid`, below
//! `MissionMarkers`, so a control measure can never occlude a marker or a slot), which is why the
//! squat is honest rather than merely convenient. **The next slice that draws zone rings must
//! MERGE into this pack rather than call `upload_hairline_segments` a second time** — a second
//! upload to one role does not add to the lane, it REPLACES it, and whichever ran last would win
//! silently. That is stated here because there is no compiler check for it.
//!
//! ══ Geometry ════════════════════════════════════════════════════════════════════════════════
//! Three kinds draw their authored vertices as a straight polyline. `curved_arrow` draws a
//! centripetal Catmull-Rom spline THROUGH them ([`catmull_rom`]), which is why the core validator
//! floors it at three points: two points have no interior control point and the spline collapses
//! to the straight segment `axis_of_advance` already is.
//!
//! Every kind that means DIRECTION (`axis_of_advance`, `curved_arrow`) gets an arrowhead at the
//! last vertex, built from the final drawn span so a curve's head follows the curve's tangent
//! rather than the chord. `boundary` gets perpendicular ticks at its authored vertices — the
//! doctrinal way a boundary is told apart from a phase line on a monochrome map, and the reason
//! the two kinds are not just two colours of the same line.
//!
//! ══ Picking ═════════════════════════════════════════════════════════════════════════════════
//! Two picks, in priority order, both point-to-SEGMENT (never point-to-infinite-line, which would
//! let a click far off the end of a short span but on its extension select it — a hit on nothing):
//! [`pick_tactical_vertex`] finds an AUTHORED vertex to drag, and [`pick_tactical_graphic`] finds
//! the whole graphic to select. The vertex pick wins where both hit, because a press on a vertex
//! is unambiguously an edit gesture and a press on the span between two vertices is not.
//!
//! Tolerances are SCREEN pixels ([`TG_PICK_PX`], [`TG_VERTEX_PICK_PX`]), unprojected to world
//! metres by the caller through the frozen press camera exactly as `CONN_PICK_PX` is — so the
//! target stays a constant size on screen at every zoom without this file learning the
//! projection's internals.

use serde_json::Value;

/// Click tolerance for [`pick_tactical_graphic`], in SCREEN pixels. Matches `CONN_PICK_PX`: a
/// hairline is 1 px and nobody can click a 1 px target.
pub(crate) const TG_PICK_PX: f64 = 6.0;

/// Click tolerance for [`pick_tactical_vertex`], in SCREEN pixels. Wider than [`TG_PICK_PX`]
/// because a vertex is a POINT rather than a run — the same reason a slot's disc is bigger than
/// the hairline that reaches it — and because the vertex pick must win over the line pick where
/// both are in range, which it cannot do reliably from a tighter radius.
pub(crate) const TG_VERTEX_PICK_PX: f64 = 9.0;

/// Samples emitted per authored span when tessellating a `curved_arrow`. Twelve is the count at
/// which a span the width of the Everon map still reads as a curve rather than a chain of chords
/// at full zoom-in, and it caps a 128-vertex graphic (the schema's `maxItems`) at ~1.5k segments —
/// two orders of magnitude under the lanes this one sits beside.
pub(crate) const CURVE_SAMPLES_PER_SPAN: usize = 12;

/// Arrowhead barb length as a fraction of the graphic's own drawn length, clamped by
/// [`ARROW_HEAD_MIN_M`] / [`ARROW_HEAD_MAX_M`]. Proportional rather than fixed so a 200 m axis and
/// a 6 km axis both read as arrows.
const ARROW_HEAD_FRAC: f64 = 0.08;
const ARROW_HEAD_MIN_M: f64 = 25.0;
const ARROW_HEAD_MAX_M: f64 = 400.0;
/// Half-angle of the arrowhead barbs, radians (~28°).
const ARROW_HEAD_ANGLE: f64 = 0.5;

/// Boundary tick half-length in metres, at the authored vertices.
const BOUNDARY_TICK_M: f64 = 40.0;

/// The SELECTED graphic: opaque amber, the hue `CONN_LINE_SELECTED_RGBA` uses for the same job on
/// the connection lane, so "this is what Delete removes" reads identically on both lanes.
pub(crate) const TG_SELECTED_RGBA: [f32; 4] = [1.0, 0.78, 0.30, 1.0];

/// Per-kind default colour, used when the author set no `style.color`. Doctrinal-ish and, more
/// importantly, mutually distinguishable at a glance on a satellite basemap.
#[must_use]
pub(crate) fn default_kind_rgba(kind: &str) -> [f32; 4] {
    match kind {
        // Phase line: the cool control-measure blue markers already use.
        "phase_line" => [0.68, 0.78, 1.0, 0.90],
        // Boundary: amber-yellow, the one hue nothing else on the mission lanes claims.
        "boundary" => [0.96, 0.88, 0.37, 0.90],
        // Axis of advance: green — a friendly scheme of manoeuvre.
        "axis_of_advance" => [0.55, 0.90, 0.55, 0.90],
        // Curved arrow: red — the kind most often used for an enemy or a counter-attack.
        "curved_arrow" => [0.95, 0.35, 0.30, 0.90],
        _ => [0.85, 0.85, 0.85, 0.85],
    }
}

/// One drawable tactical graphic, resolved from the document.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TacticalGraphic {
    /// `tacticalGraphics[].id`, the same id the pick returns and the delete consumes.
    pub(crate) id: String,
    /// One of `map_engine_core::mission::tactical_graphics::KINDS`.
    pub(crate) kind: String,
    /// The AUTHORED vertices — what a drag edits, and what [`pick_tactical_vertex`] hit-tests.
    /// Never the tessellated curve.
    pub(crate) points: Vec<[f64; 2]>,
    /// `label`, empty when unauthored. Carried so a later label lane has the read it needs; this
    /// module draws no text (there is no text lane to draw it in without a new render lane).
    pub(crate) label: String,
    /// Resolved stroke colour: `style.color`/`style.alpha` when authored, else
    /// [`default_kind_rgba`].
    pub(crate) rgba: [f32; 4],
}

/// Parse the graphics out of an editor `meta.environment` bag.
///
/// **Lenient by construction, and that is deliberate.** This is a DRAW path over a live document
/// the author is mid-way through editing: a row that is malformed is SKIPPED, never a panic and
/// never a fabricated default. The refusal with a sentence belongs at the compile boundary
/// (`map_engine_core::mission::tactical_graphics::validate`, which the `AUTHORED_BLOCKS` row
/// runs), where the author can read it — the same split `copy_authored_blocks` documents for the
/// save path.
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
        // A single vertex is not a line and has nothing to draw; skipping it here is the same
        // "don't draw a lie" rule `connection_segments` applies to a dangling edge.
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

/// `#rrggbb` → linear-ish 0..1 RGB. `None` for anything the schema's `$defs/hexColor` would
/// refuse, so a half-typed colour falls back to the kind default instead of drawing black.
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

/// Centripetal Catmull-Rom through `points`, `per_span` samples per authored span.
///
/// CENTRIPETAL (alpha = 0.5), not uniform. The uniform parameterisation is one line shorter and
/// forms a CUSP or a self-intersecting loop whenever three authored vertices are unevenly spaced —
/// which is what hand-drawn map vertices always are, so the uniform form would misdraw the common
/// case rather than a corner one.
///
/// The endpoints are duplicated to synthesise the two phantom control points the spline needs, so
/// the curve starts and ends exactly on the authored first and last vertices. Fewer than two
/// points returns them unchanged: there is nothing to interpolate and inventing a curve would be a
/// fabricated default.
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
        // Centripetal knot spacing: t_{k+1} = t_k + |p_{k+1} - p_k|^0.5.
        let knot = |t: f64, a: [f64; 2], b: [f64; 2]| -> f64 {
            let d = (b[0] - a[0]).hypot(b[1] - a[1]);
            // A repeated point would make two knots equal and divide by zero; nudging by an
            // epsilon keeps the segment straight there instead of producing NaN vertices, which
            // the GPU would draw as a wild spike rather than as nothing.
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

/// The polyline actually DRAWN for `g`: the authored vertices for the three straight kinds, the
/// Catmull-Rom tessellation for `curved_arrow`.
#[must_use]
pub(crate) fn drawn_polyline(g: &TacticalGraphic) -> Vec<[f64; 2]> {
    if g.kind == "curved_arrow" {
        catmull_rom(&g.points, CURVE_SAMPLES_PER_SPAN)
    } else {
        g.points.clone()
    }
}

/// Does this kind carry an arrowhead? Direction-bearing kinds do; a phase line and a boundary are
/// not directed and an arrowhead on either would assert something the author did not.
#[must_use]
pub(crate) fn kind_has_arrowhead(kind: &str) -> bool {
    matches!(kind, "axis_of_advance" | "curved_arrow")
}

/// The two barb segments of the arrowhead at the END of `poly`, or empty when there is no
/// direction to point in (fewer than two points, or a zero-length final span).
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
    // Unit vector pointing BACK along the final span — the barbs are rotated off this.
    let (ux, uy) = (-dx / span, -dy / span);
    let (c, s) = (ARROW_HEAD_ANGLE.cos(), ARROW_HEAD_ANGLE.sin());
    let barb = |sign: f64| -> [f64; 2] {
        let rx = ux.mul_add(c, -(uy * s * sign));
        let ry = (ux * s).mul_add(sign, uy * c);
        [rx.mul_add(len, tip[0]), ry.mul_add(len, tip[1])]
    };
    vec![(tip, barb(1.0)), (tip, barb(-1.0))]
}

/// Perpendicular tick segments at each AUTHORED vertex of a boundary.
#[must_use]
fn boundary_ticks(points: &[[f64; 2]]) -> Vec<([f64; 2], [f64; 2])> {
    let mut out = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        // The local direction: the span leaving this vertex, or the one arriving at the last.
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

/// Every drawable segment of `g`, in draw order: the polyline spans, then the kind's ornament
/// (arrowhead barbs or boundary ticks).
///
/// Shared by [`tactical_lane_verts`] and by nothing else on purpose — a second producer is how the
/// drawn set and the picked set drift apart, which is exactly what the module header refuses.
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

/// Pack the graphics into the flat `[x,y,r,g,b,a]` LineList
/// `RenderEngine::upload_hairline_segments` takes: 6 floats per vertex, 2 vertices per segment, in
/// `graphics` order.
///
/// `selected` tints exactly one graphic, matched by id against the same ids
/// [`pick_tactical_graphic`] returns and the delete consumes — so the highlighted line and the line
/// Delete removes are the same line by construction rather than by convention (T-780's rule).
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

/// Segment count for the packed buffer — `verts.len() / 12`, stated once so no caller re-derives
/// it and gets it wrong.
#[must_use]
pub(crate) fn lane_segment_count(verts: &[f32]) -> u32 {
    #[allow(clippy::cast_possible_truncation)]
    {
        (verts.len() / 12) as u32
    }
}

/// Squared point-to-segment distance, the shared kernel of both picks.
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

/// The graphic under a world point, or `None`. `tol_m` is the click radius in world metres (the
/// caller converts [`TG_PICK_PX`] through the frozen press camera). Nearest wins, so overlapping
/// graphics resolve deterministically instead of by listing order.
///
/// Tested against the DRAWN segments — including a curve's tessellation and the ornaments — so a
/// click on the visible line always finds the graphic the operator can see, never the invisible
/// chord between two authored vertices of a curve.
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

/// The AUTHORED vertex under a world point as `(graphic id, vertex index)`, or `None`.
///
/// Authored vertices only — a curve's tessellated samples are not draggable, because moving one
/// would have nowhere to be written back to. Nearest wins.
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

/* ══════════ The DRAW state machine's pure half ═══════════════════════════════════════════════
 *
 * `state/operations/tactical_graphics.rs` owns the in-flight draw, but that whole module is
 * `#![cfg(target_arch = "wasm32")]` (it reaches the document through `OPS_CTX`'s `!Send` `Rc`s), so
 * a `#[cfg(test)]` block there is compiled by NOTHING on the native test runner — it would report
 * a green that examined no code, which is the one defect this program exists to kill. So the parts
 * with real arithmetic live HERE, where `cargo test -p website-frontend` actually runs them, and
 * the wasm module is left as thin glue.
 */

/// An in-flight multi-click draw.
#[derive(Clone, Debug, PartialEq)]
pub struct TacticalDraft {
    /// The armed kind — one of `map_engine_core::mission::tactical_graphics::KINDS`.
    pub kind: String,
    /// Vertices placed so far, in click order.
    pub verts: Vec<(f64, f64)>,
}

impl TacticalDraft {
    /// How many more vertices before the draw may commit.
    ///
    /// Delegates to the CORE validator's own floor rather than restating it, so the canvas can
    /// never complete a graphic `/compiled` would refuse — the two numbers are one function.
    #[must_use]
    pub fn needed(&self) -> usize {
        map_engine_core::mission::tactical_graphics::min_points(&self.kind)
            .unwrap_or(2)
            .saturating_sub(self.verts.len())
    }

    // NO `hint()` HELPER HERE. A "3 vertices — one more needed" string wants a dock row to render
    // it, and the only surfaces that could are under `panels/`, which this slice does not own —
    // so the method would be dead code that only its own test ever called, which is the shape a
    // reader mistakes for a live feature. `needed()` is the fact; the wording is the caller's.
}

/// A stable id unique within `rows`: `tg_{kind initials}_{n}`, `n` the first free index.
///
/// Minted against the LIVE rows rather than from a monotonic counter, so an id can never collide
/// after an undo has removed the row a counter would have skipped past — the hazard
/// `OpsCtx::next_id` guards against by re-proving uniqueness against the document.
#[must_use]
pub(crate) fn mint_graphic_id(rows: &[Value], kind: &str) -> String {
    let stem: String = kind.split('_').filter_map(|w| w.chars().next()).collect();
    for n in 1..=u32::MAX {
        let candidate = format!("tg_{stem}_{n}");
        let taken = rows
            .iter()
            .any(|r| r.get("id").and_then(Value::as_str) == Some(candidate.as_str()));
        if !taken {
            return candidate;
        }
    }
    // Unreachable in practice (the schema caps the block at 128 rows long before 2^32); the stem
    // alone is still a legal non-empty id rather than a panic on a live document.
    format!("tg_{stem}")
}

/// The live graphics for `core` — the wasm-side document read.
///
/// Reads `meta.environment.tacticalGraphics` out of `small_maps_json`, the SAME projection
/// `editor_ops::read_env_value` reads and `compile_payload`'s `copy_authored_blocks` promotes from,
/// so the canvas and the compiled document can never disagree about what was authored.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub(crate) fn live_tactical_graphics(
    core: &map_engine_core::doc::MissionDocCore,
) -> Vec<TacticalGraphic> {
    let Ok(root) = serde_json::from_str::<Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    root.get("meta")
        .and_then(|m| m.get("environment"))
        .map_or_else(Vec::new, tactical_graphics_from_env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn env(rows: Value) -> Value {
        json!({"weather": "clear", "tacticalGraphics": rows})
    }

    fn one_of_each() -> Value {
        env(json!([
            {"id": "pl", "kind": "phase_line", "points": [[0.0, 0.0], [100.0, 0.0]],
             "label": "PL BLUE"},
            {"id": "bd", "kind": "boundary", "points": [[0.0, 500.0], [100.0, 500.0], [200.0, 600.0]]},
            {"id": "ax", "kind": "axis_of_advance", "points": [[0.0, 1000.0], [1000.0, 1000.0]]},
            {"id": "ar", "kind": "curved_arrow",
             "points": [[0.0, 2000.0], [500.0, 2200.0], [1000.0, 2000.0]]}
        ]))
    }

    #[test]
    fn all_four_kinds_parse_and_keep_their_authored_vertices() {
        let gs = tactical_graphics_from_env(&one_of_each());
        assert_eq!(gs.len(), 4);
        assert_eq!(gs[0].id, "pl");
        assert_eq!(gs[0].kind, "phase_line");
        assert_eq!(gs[0].label, "PL BLUE");
        assert_eq!(gs[0].points, vec![[0.0, 0.0], [100.0, 0.0]]);
        assert_eq!(gs[1].points.len(), 3);
        assert_eq!(gs[3].kind, "curved_arrow");
    }

    #[test]
    fn every_kind_produces_drawable_segments_and_the_lane_packs_them() {
        let gs = tactical_graphics_from_env(&one_of_each());
        for g in &gs {
            assert!(
                !graphic_segments(g).is_empty(),
                "{} must draw something",
                g.kind
            );
        }
        let verts = tactical_lane_verts(&gs, None);
        assert_eq!(verts.len() % 12, 0, "6 floats/vertex, 2 vertices/segment");
        let total: usize = gs.iter().map(|g| graphic_segments(g).len()).sum();
        assert_eq!(lane_segment_count(&verts) as usize, total);
        assert!(tactical_lane_verts(&[], None).is_empty());
    }

    /// The two directed kinds get barbs and the two undirected ones do not — the claim the module
    /// header makes in prose, asserted in segment counts.
    #[test]
    fn only_the_directed_kinds_carry_an_arrowhead_and_only_a_boundary_carries_ticks() {
        assert!(kind_has_arrowhead("axis_of_advance"));
        assert!(kind_has_arrowhead("curved_arrow"));
        assert!(!kind_has_arrowhead("phase_line"));
        assert!(!kind_has_arrowhead("boundary"));

        let gs = tactical_graphics_from_env(&one_of_each());
        let by = |id: &str| gs.iter().find(|g| g.id == id).expect("row").clone();

        // A 2-point phase line is exactly one span, no ornament.
        assert_eq!(graphic_segments(&by("pl")).len(), 1);
        // A 2-point axis is one span plus two barbs.
        assert_eq!(graphic_segments(&by("ax")).len(), 3);
        // A 3-point boundary is two spans plus three ticks, and no barbs.
        assert_eq!(graphic_segments(&by("bd")).len(), 5);
        // A 3-point curve tessellates to 2 spans x CURVE_SAMPLES_PER_SPAN, plus two barbs.
        assert_eq!(
            graphic_segments(&by("ar")).len(),
            2 * CURVE_SAMPLES_PER_SPAN + 2
        );
    }

    #[test]
    fn the_curve_passes_through_its_authored_vertices_and_bows_off_the_chord() {
        let pts = [[0.0, 0.0], [500.0, 200.0], [1000.0, 0.0]];
        let curve = catmull_rom(&pts, CURVE_SAMPLES_PER_SPAN);
        assert_eq!(curve.len(), 2 * CURVE_SAMPLES_PER_SPAN + 1);
        assert_eq!(curve[0], pts[0]);
        assert_eq!(*curve.last().expect("non-empty"), pts[2]);
        assert!(
            curve
                .iter()
                .any(|p| (p[0] - pts[1][0]).abs() < 1.0 && (p[1] - pts[1][1]).abs() < 1.0),
            "the spline must pass through the interior authored vertex"
        );
        // A bow, not a chord: the chord from (0,0) to (1000,0) is y = 0 everywhere.
        assert!(
            curve.iter().any(|p| p[1] > 20.0),
            "a Catmull-Rom through a raised middle vertex must leave the chord: {curve:?}"
        );
        // Every sample is finite — the repeated-point epsilon guard, driven directly.
        let doubled = [[0.0, 0.0], [0.0, 0.0], [10.0, 10.0], [10.0, 10.0]];
        assert!(
            catmull_rom(&doubled, 4)
                .iter()
                .all(|p| p[0].is_finite() && p[1].is_finite()),
            "repeated vertices must not produce NaN"
        );
        // Too few points is returned unchanged rather than invented into a curve.
        assert_eq!(catmull_rom(&pts[..2], 8), pts[..2].to_vec());
    }

    #[test]
    fn the_line_pick_is_point_to_segment_and_nearest_wins() {
        let gs = tactical_graphics_from_env(&env(json!([
            {"id": "a", "kind": "phase_line", "points": [[0.0, 0.0], [100.0, 0.0]]},
            {"id": "b", "kind": "phase_line", "points": [[0.0, 20.0], [100.0, 20.0]]}
        ])));
        assert_eq!(
            pick_tactical_graphic(&gs, 50.0, 3.0, 6.0).as_deref(),
            Some("a")
        );
        assert_eq!(
            pick_tactical_graphic(&gs, 50.0, 17.0, 6.0).as_deref(),
            Some("b")
        );
        // Nearest wins where both are in range.
        assert_eq!(
            pick_tactical_graphic(&gs, 50.0, 12.0, 30.0).as_deref(),
            Some("b")
        );
        // Off the END of the span but on its infinite extension: a miss, not a hit on nothing.
        assert_eq!(pick_tactical_graphic(&gs, 200.0, 0.0, 6.0), None);
        assert_eq!(pick_tactical_graphic(&gs, 50.0, 60.0, 6.0), None);
        assert_eq!(pick_tactical_graphic(&[], 0.0, 0.0, 1e6), None);
    }

    /// A click on the VISIBLE curve finds the graphic even where the chord between the authored
    /// vertices is far away — the reason the pick runs over `graphic_segments` and not `points`.
    #[test]
    fn the_pick_follows_the_drawn_curve_not_the_authored_chord() {
        let gs = tactical_graphics_from_env(&env(json!([
            {"id": "ar", "kind": "curved_arrow",
             "points": [[0.0, 0.0], [500.0, 300.0], [1000.0, 0.0]]}
        ])));
        let curve = drawn_polyline(&gs[0]);
        assert_eq!(
            curve.len(),
            2 * CURVE_SAMPLES_PER_SPAN + 1,
            "the curve must be TESSELLATED, not the three authored vertices"
        );
        // The probe is the drawn point FURTHEST from both authored chords. Taking the apex of the
        // curve instead would be a probe derived from the function under test in a way that
        // follows it anywhere: a tessellation that silently collapsed to the chords would still
        // put its highest point on a chord and still be "hit".
        let clearance = |p: [f64; 2]| {
            dist_to_segment(p[0], p[1], [0.0, 0.0], [500.0, 300.0]).min(dist_to_segment(
                p[0],
                p[1],
                [500.0, 300.0],
                [1000.0, 0.0],
            ))
        };
        let off_chord = curve
            .iter()
            .copied()
            .max_by(|a, b| clearance(*a).total_cmp(&clearance(*b)))
            .expect("curve has samples");
        assert!(
            clearance(off_chord) > 20.0,
            "the spline must leave the authored chords by more than the pick tolerance, else \
             this test cannot tell a curve from a chord: {off_chord:?} clear {}",
            clearance(off_chord)
        );
        assert_eq!(
            pick_tactical_graphic(&gs, off_chord[0], off_chord[1], 5.0).as_deref(),
            Some("ar"),
            "a click on the drawn curve, off the authored chords, must hit"
        );
        // ...and the SAME click must MISS a straight kind through the identical vertices. That is
        // what proves the hit above came from the tessellation rather than from a loose tolerance.
        let straight = tactical_graphics_from_env(&env(json!([
            {"id": "ax", "kind": "axis_of_advance",
             "points": [[0.0, 0.0], [500.0, 300.0], [1000.0, 0.0]]}
        ])));
        assert_eq!(
            pick_tactical_graphic(&straight, off_chord[0], off_chord[1], 5.0),
            None,
            "the straight kind draws the chords, so the off-chord probe must miss it"
        );
    }

    #[test]
    fn the_vertex_pick_returns_authored_indices_only() {
        let gs = tactical_graphics_from_env(&one_of_each());
        assert_eq!(
            pick_tactical_vertex(&gs, 100.0, 0.0, 5.0),
            Some(("pl".to_string(), 1))
        );
        assert_eq!(
            pick_tactical_vertex(&gs, 200.0, 600.0, 5.0),
            Some(("bd".to_string(), 2))
        );
        assert_eq!(pick_tactical_vertex(&gs, 50.0, 0.0, 5.0), None, "mid-span");
        // A curve's tessellated samples are NOT vertices: the apex of `ar` is a drawn point but
        // has no authored index, so it must not be draggable.
        let apex = drawn_polyline(&gs[3])
            .into_iter()
            .max_by(|a, b| a[1].total_cmp(&b[1]))
            .expect("samples");
        let hit = pick_tactical_vertex(&gs, apex[0], apex[1], 5.0);
        assert!(
            hit.is_none() || hit.as_ref().is_some_and(|(_, i)| *i == 1),
            "only an authored index may come back: {hit:?}"
        );
    }

    #[test]
    fn selection_tints_exactly_one_graphic_and_style_overrides_the_kind_default() {
        let gs = tactical_graphics_from_env(&env(json!([
            {"id": "a", "kind": "phase_line", "points": [[0.0, 0.0], [100.0, 0.0]]},
            {"id": "b", "kind": "phase_line", "points": [[0.0, 20.0], [100.0, 20.0]],
             "style": {"color": "#ff0000", "alpha": 0.5}}
        ])));
        assert_eq!(gs[0].rgba, default_kind_rgba("phase_line"));
        assert_eq!(gs[1].rgba, [1.0, 0.0, 0.0, 0.5]);

        let v = tactical_lane_verts(&gs, Some("b"));
        // Segment 0 is `a` (kind default), segment 1 is `b` (selected amber).
        assert_eq!(&v[2..6], &default_kind_rgba("phase_line"));
        assert_eq!(&v[14..18], &TG_SELECTED_RGBA);
        // A selection naming nothing tints nothing — the stale-selection case.
        let none = tactical_lane_verts(&gs, Some("gone"));
        assert_eq!(&none[14..18], &gs[1].rgba);
    }

    #[test]
    fn a_malformed_row_is_skipped_rather_than_drawn_wrong() {
        let gs = tactical_graphics_from_env(&env(json!([
            {"id": "", "kind": "phase_line", "points": [[0.0, 0.0], [1.0, 1.0]]},
            {"id": "no-kind", "points": [[0.0, 0.0], [1.0, 1.0]]},
            {"id": "short", "kind": "phase_line", "points": [[0.0, 0.0]]},
            {"id": "junk", "kind": "phase_line", "points": "nope"},
            {"id": "ok", "kind": "phase_line", "points": [[0.0, 0.0], [1.0, 1.0], ["x", 2.0]]}
        ])));
        assert_eq!(gs.len(), 1, "{gs:?}");
        assert_eq!(gs[0].id, "ok");
        assert_eq!(
            gs[0].points.len(),
            2,
            "the bad vertex is dropped, not zeroed"
        );

        assert!(tactical_graphics_from_env(&json!({"weather": "clear"})).is_empty());
        assert!(tactical_graphics_from_env(&json!(null)).is_empty());
        assert!(tactical_graphics_from_env(&env(json!("not an array"))).is_empty());
    }

    /// The draft's arithmetic — what gates the commit, and the reason it lives in this file at
    /// all (see the "DRAW state machine's pure half" note above).
    #[test]
    fn a_draft_knows_how_many_vertices_it_still_needs() {
        let mut d = TacticalDraft {
            kind: "phase_line".to_string(),
            verts: Vec::new(),
        };
        assert_eq!(d.needed(), 2);
        d.verts.push((0.0, 0.0));
        assert_eq!(d.needed(), 1);
        d.verts.push((1.0, 1.0));
        assert_eq!(d.needed(), 0);
        // Past the floor stays at zero — `saturating_sub`, so an extra vertex never wraps to a
        // huge "still needed" count and locks the commit out.
        d.verts.push((2.0, 2.0));
        assert_eq!(d.needed(), 0);

        let c = TacticalDraft {
            kind: "curved_arrow".to_string(),
            verts: vec![(0.0, 0.0), (1.0, 1.0)],
        };
        assert_eq!(c.needed(), 1, "two points cannot curve");
    }

    /// The canvas floor and the compile floor are ONE function, walked over the core's own
    /// vocabulary so a kind added there without one here fails by name.
    #[test]
    fn the_canvas_floor_is_the_core_validator_floor() {
        use map_engine_core::mission::tactical_graphics::{min_points, KINDS};
        assert_eq!(KINDS.len(), 4);
        for kind in KINDS {
            let floor = min_points(kind).unwrap_or_else(|| panic!("{kind} has no floor"));
            let draft = TacticalDraft {
                kind: (*kind).to_string(),
                verts: vec![(0.0, 0.0); floor],
            };
            assert_eq!(draft.needed(), 0, "{kind} must commit at its own floor");
            let short = TacticalDraft {
                kind: (*kind).to_string(),
                verts: vec![(0.0, 0.0); floor - 1],
            };
            assert_eq!(short.needed(), 1, "{kind} must refuse one under it");
        }
    }

    #[test]
    fn a_minted_id_never_collides_with_a_live_row() {
        let rows = vec![
            json!({"id": "tg_pl_1"}),
            json!({"id": "tg_pl_2"}),
            json!({"id": "tg_ca_1"}),
        ];
        assert_eq!(mint_graphic_id(&rows, "phase_line"), "tg_pl_3");
        assert_eq!(mint_graphic_id(&rows, "curved_arrow"), "tg_ca_2");
        assert_eq!(mint_graphic_id(&rows, "axis_of_advance"), "tg_aoa_1");
        assert_eq!(mint_graphic_id(&[], "boundary"), "tg_b_1");
    }

    #[test]
    fn a_bad_hex_colour_falls_back_to_the_kind_default() {
        assert_eq!(hex_to_rgb("#ff8000"), Some([1.0, 128.0 / 255.0, 0.0]));
        assert_eq!(hex_to_rgb("ff8000"), None);
        assert_eq!(hex_to_rgb("#ff800"), None);
        assert_eq!(hex_to_rgb("#gggggg"), None);
        let gs = tactical_graphics_from_env(&env(json!([
            {"id": "a", "kind": "boundary", "points": [[0.0, 0.0], [1.0, 0.0]],
             "style": {"color": "nope", "alpha": 4.0}}
        ])));
        assert_eq!(
            gs[0].rgba,
            default_kind_rgba("boundary"),
            "an unusable style must not draw black at alpha 4"
        );
    }
}
