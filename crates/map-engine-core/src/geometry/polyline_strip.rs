//! Meter-width polyline strip expansion for road casing/centerline (T-151.4 L2/L9, T-151.4.1 joins).
//!
//! Deck `PathLayer` draws strokes in world meters (`widthUnits: 'meters'`, `capRounded` +
//! `jointRounded`). On wgpu/WebGL2 native line width is 1 px, so roads are expanded on the CPU
//! into a triangle-list strip with **miter joins** and **round end caps**.
//!
//! **L9 gate:** projected screen width at a segment midpoint == `width_m · 2^deckZoom` px ± 1e-6.

/// One triangle-list vertex: position + RGBA (normalized 0..1). Layout matches `lanes::LineVertex`
/// / polygon fill (24 B) so the same `PolygonFill` pipeline can draw road strips.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

/// Normalize an RGBA u8 palette to linear 0..1 (luma.gl-style).
#[must_use]
pub fn norm_rgba(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

/// Screen-space width in pixels for a world-meter stroke at deckZoom (L9 contract).
#[must_use]
pub fn projected_width_px(width_m: f64, deck_zoom: f64) -> f64 {
    width_m * 2.0_f64.powf(deck_zoom)
}

/// Miter length limit as a multiple of half-width (bevel when exceeded) — prevents spikes on
/// acute corners; similar spirit to SVG stroke-miterlimit ≈ 4.
const MITER_LIMIT: f64 = 4.0;
/// Segments in a round end-cap semicircle.
const CAP_SEGMENTS: usize = 8;

#[inline]
fn v_sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

#[inline]
fn v_add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

#[inline]
fn v_scale(a: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] * s, a[1] * s]
}

#[inline]
fn v_len(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}

#[inline]
fn v_norm(a: [f64; 2]) -> Option<[f64; 2]> {
    let l = v_len(a);
    if l < 1e-12 {
        None
    } else {
        Some([a[0] / l, a[1] / l])
    }
}

/// Left-hand perpendicular of a unit direction (CCW).
#[inline]
fn perp_left(d: [f64; 2]) -> [f64; 2] {
    [-d[1], d[0]]
}

fn push_tri(out: &mut Vec<StripVertex>, a: [f64; 2], b: [f64; 2], c: [f64; 2], color: [f32; 4]) {
    for p in [a, b, c] {
        out.push(StripVertex {
            pos: [p[0] as f32, p[1] as f32],
            color,
        });
    }
}

/// Expand a polyline centerline into a joined triangle-list strip (miter joins + round caps).
///
/// `points` are world meters; output positions are still world meters (engine subtracts ANCHOR).
#[must_use]
pub fn expand_polyline_strip(
    points: &[[f64; 2]],
    width_m: f64,
    color: [f32; 4],
) -> Vec<StripVertex> {
    if points.len() < 2 || width_m <= 0.0 {
        return Vec::new();
    }
    let half = width_m * 0.5;

    // Collapse near-duplicates so join math stays stable.
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(points.len());
    for &p in points {
        if let Some(prev) = pts.last()
            && (p[0] - prev[0]).hypot(p[1] - prev[1]) < 1e-9
        {
            continue;
        }
        pts.push(p);
    }
    if pts.len() < 2 {
        return Vec::new();
    }

    let n = pts.len();
    // Unit directions of each segment i → i+1.
    let mut dirs: Vec<[f64; 2]> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let Some(d) = v_norm(v_sub(pts[i + 1], pts[i])) else {
            // Degenerate segment — drop by merging points already handled; use zero dir skip.
            dirs.push([0.0, 0.0]);
            continue;
        };
        dirs.push(d);
    }

    // Per-vertex left/right offset positions (miter or bevel).
    let mut left = vec![[0.0_f64; 2]; n];
    let mut right = vec![[0.0_f64; 2]; n];

    for i in 0..n {
        if i == 0 {
            let d = dirs[0];
            if v_len(d) < 0.5 {
                left[i] = pts[i];
                right[i] = pts[i];
                continue;
            }
            let nl = perp_left(d);
            left[i] = v_add(pts[i], v_scale(nl, half));
            right[i] = v_add(pts[i], v_scale(nl, -half));
            continue;
        }
        if i == n - 1 {
            let d = dirs[n - 2];
            if v_len(d) < 0.5 {
                left[i] = pts[i];
                right[i] = pts[i];
                continue;
            }
            let nl = perp_left(d);
            left[i] = v_add(pts[i], v_scale(nl, half));
            right[i] = v_add(pts[i], v_scale(nl, -half));
            continue;
        }

        let d0 = dirs[i - 1];
        let d1 = dirs[i];
        if v_len(d0) < 0.5 || v_len(d1) < 0.5 {
            let d = if v_len(d0) >= 0.5 { d0 } else { d1 };
            let nl = perp_left(d);
            left[i] = v_add(pts[i], v_scale(nl, half));
            right[i] = v_add(pts[i], v_scale(nl, -half));
            continue;
        }

        let n0 = perp_left(d0);
        let n1 = perp_left(d1);
        // Miter vector: average of normals, scaled so projection on n0 equals half.
        let m = v_add(n0, n1);
        let ml = v_len(m);
        if ml < 1e-12 {
            // 180° fold — use either normal.
            left[i] = v_add(pts[i], v_scale(n0, half));
            right[i] = v_add(pts[i], v_scale(n0, -half));
            continue;
        }
        let m_hat = [m[0] / ml, m[1] / ml];
        // cos(θ/2) = n0 · m_hat; miter length = half / cos(θ/2)
        let cos_half = n0[0] * m_hat[0] + n0[1] * m_hat[1];
        if cos_half.abs() < 1e-6 {
            left[i] = v_add(pts[i], v_scale(n0, half));
            right[i] = v_add(pts[i], v_scale(n0, -half));
            continue;
        }
        let miter_len = half / cos_half.abs();
        if miter_len > half * MITER_LIMIT {
            // Bevel: use each segment's offset at the vertex (two outer points — we pick the
            // average outer for a simple continuous strip; a true bevel is two tris).
            left[i] = v_add(pts[i], v_scale(n0, half * cos_half.signum()));
            // Prefer n1 for continuity of next segment.
            left[i] = v_add(
                pts[i],
                v_scale(
                    v_norm(v_add(n0, n1)).unwrap_or(n0),
                    half * if cos_half >= 0.0 { 1.0 } else { -1.0 },
                ),
            );
            right[i] = v_add(pts[i], v_scale(v_norm(v_add(n0, n1)).unwrap_or(n0), -half));
        } else {
            let sign = if cos_half >= 0.0 { 1.0 } else { -1.0 };
            left[i] = v_add(pts[i], v_scale(m_hat, miter_len * sign));
            right[i] = v_add(pts[i], v_scale(m_hat, -miter_len * sign));
        }
    }

    let mut out = Vec::with_capacity((n - 1) * 6 + CAP_SEGMENTS * 6 * 2);

    // Segment quads using joined endpoints.
    for i in 0..n - 1 {
        if v_len(dirs[i]) < 0.5 {
            continue;
        }
        let a_l = left[i];
        let a_r = right[i];
        let b_l = left[i + 1];
        let b_r = right[i + 1];
        push_tri(&mut out, a_l, a_r, b_r, color);
        push_tri(&mut out, a_l, b_r, b_l, color);
    }

    // Round caps at start / end.
    add_round_cap(&mut out, pts[0], dirs[0], half, true, color);
    add_round_cap(&mut out, pts[n - 1], dirs[n - 2], half, false, color);

    out
}

/// Semicircle cap at endpoint. `at_start`: fan outward opposite to `dir`; else along `dir`.
fn add_round_cap(
    out: &mut Vec<StripVertex>,
    center: [f64; 2],
    dir: [f64; 2],
    half: f64,
    at_start: bool,
    color: [f32; 4],
) {
    if v_len(dir) < 0.5 {
        return;
    }
    let outward = if at_start { v_scale(dir, -1.0) } else { dir };
    let left = perp_left(if at_start { outward } else { dir });
    // Fan from left → through outward → right (semicircle).
    let start_ang = left;
    // Rotate from +left to -left through outward (π radians).
    for k in 0..CAP_SEGMENTS {
        let t0 = std::f64::consts::PI * (k as f64) / (CAP_SEGMENTS as f64);
        let t1 = std::f64::consts::PI * ((k + 1) as f64) / (CAP_SEGMENTS as f64);
        let p0 = rot_scale(start_ang, outward, t0, half);
        let p1 = rot_scale(start_ang, outward, t1, half);
        push_tri(out, center, v_add(center, p0), v_add(center, p1), color);
    }
}

/// Rotate `start` toward `outward` basis: angle 0 = start (left), π/2 = outward, π = -start.
fn rot_scale(left: [f64; 2], outward: [f64; 2], angle: f64, half: f64) -> [f64; 2] {
    let c = angle.cos();
    let s = angle.sin();
    // left * cos + outward * sin
    v_scale(
        [left[0] * c + outward[0] * s, left[1] * c + outward[1] * s],
        half,
    )
}

/// Dash pattern [dash, gap] in meters (mirror `dashArrayFor`: dashed roads = [8, 6]).
/// Emits only the dash intervals of the path as separate joined strips.
#[must_use]
pub fn expand_dashed_polyline_strip(
    points: &[[f64; 2]],
    width_m: f64,
    color: [f32; 4],
    dash_m: f64,
    gap_m: f64,
) -> Vec<StripVertex> {
    if points.len() < 2 || dash_m <= 0.0 {
        return expand_polyline_strip(points, width_m, color);
    }
    let period = dash_m + gap_m;
    if period <= 0.0 {
        return expand_polyline_strip(points, width_m, color);
    }

    let mut out = Vec::new();
    let mut in_dash = true;
    let mut phase = 0.0_f64;

    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let seg_len = (b[0] - a[0]).hypot(b[1] - a[1]);
        if seg_len < 1e-12 {
            continue;
        }
        let dir = [(b[0] - a[0]) / seg_len, (b[1] - a[1]) / seg_len];
        let mut t = 0.0_f64;
        while t < seg_len - 1e-12 {
            let phase_len = if in_dash { dash_m } else { gap_m };
            let remain_phase = phase_len - phase;
            let remain_seg = seg_len - t;
            let step = remain_phase.min(remain_seg);
            if in_dash && step > 1e-12 {
                let p0 = [a[0] + dir[0] * t, a[1] + dir[1] * t];
                let p1 = [a[0] + dir[0] * (t + step), a[1] + dir[1] * (t + step)];
                out.extend(expand_polyline_strip(&[p0, p1], width_m, color));
            }
            t += step;
            phase += step;
            if phase + 1e-12 >= phase_len {
                phase = 0.0;
                in_dash = !in_dash;
            }
        }
    }
    out
}

/// Road casing color (near-black) — `roadLayer.ts` casing underlay.
pub const ROAD_CASING_RGBA: [u8; 4] = [30, 30, 34, 255];
/// Casing width factor over measured centerline width — `roadLayer.ts` × 1.4.
pub const ROAD_CASING_FACTOR: f64 = 1.4;

/// Style table entry (mirror `ROAD_STYLES`).
#[derive(Clone, Copy, Debug)]
pub struct RoadStyle {
    pub color: [u8; 3],
    pub fallback_width_m: f64,
    pub dashed: bool,
}

/// Closed `ROAD_STYLES` enum (`roadLayer.ts:39`).
#[must_use]
pub fn road_style(road_class: &str) -> Option<RoadStyle> {
    Some(match road_class {
        "highway_paved" => RoadStyle {
            color: [0xc8, 0xc8, 0xc8],
            fallback_width_m: 4.0,
            dashed: false,
        },
        "road_paved" => RoadStyle {
            color: [0xa0, 0xa0, 0xa0],
            fallback_width_m: 2.5,
            dashed: false,
        },
        "road_dirt" => RoadStyle {
            color: [0x8b, 0x69, 0x14],
            fallback_width_m: 2.0,
            dashed: true,
        },
        "track" => RoadStyle {
            color: [0x6b, 0x50, 0x10],
            fallback_width_m: 1.5,
            dashed: true,
        },
        "path" => RoadStyle {
            color: [0x5a, 0x4a, 0x3a],
            fallback_width_m: 1.0,
            dashed: true,
        },
        "runway" => RoadStyle {
            color: [0xff, 0xff, 0xff],
            fallback_width_m: 6.0,
            dashed: false,
        },
        _ => return None,
    })
}

/// Min deckZoom gate per road class (mirror `lodGates.ts` MIN_ZOOM_GATES).
#[must_use]
pub fn road_class_visible(road_class: &str, deck_zoom: f64) -> bool {
    let min = match road_class {
        "highway_paved" | "road_paved" | "runway" => -6.0,
        "road_dirt" | "track" => -2.0,
        "path" => 4.0,
        _ => return false,
    };
    deck_zoom >= min
}

/// Compose casing + centerline strip vertices for one road segment.
/// Returns `(casing_verts, centerline_verts)`.
#[must_use]
pub fn compose_road_segment(
    points: &[[f64; 2]],
    width_m: f64,
    road_class: &str,
) -> (Vec<StripVertex>, Vec<StripVertex>) {
    let Some(style) = road_style(road_class) else {
        return (Vec::new(), Vec::new());
    };
    let w = if width_m > 0.3 && width_m < 40.0 {
        width_m
    } else {
        style.fallback_width_m
    };
    let center_color = norm_rgba([style.color[0], style.color[1], style.color[2], 255]);
    let casing_color = norm_rgba(ROAD_CASING_RGBA);
    let casing = expand_polyline_strip(points, w * ROAD_CASING_FACTOR, casing_color);
    let center = if style.dashed {
        expand_dashed_polyline_strip(points, w, center_color, 8.0, 6.0)
    } else {
        expand_polyline_strip(points, w, center_color)
    };
    (casing, center)
}

/// Pack strip vertices into a flat f32 buffer: `[x,y,r,g,b,a]…` (6 f32 per vertex) in **world**
/// meters — the engine converts to anchor-relative on upload.
#[must_use]
pub fn pack_strip_verts(verts: &[StripVertex]) -> Vec<f32> {
    let mut out = Vec::with_capacity(verts.len() * 6);
    for v in verts {
        out.push(v.pos[0]);
        out.push(v.pos[1]);
        out.push(v.color[0]);
        out.push(v.color[1]);
        out.push(v.color[2]);
        out.push(v.color[3]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L9: projected screen width == widthM · 2^deckZoom ± 1e-6.
    #[test]
    fn polyline_width_midpoint_projection() {
        let width_m = 4.0_f64;
        let zoom = -2.0_f64;
        let expected_px = projected_width_px(width_m, zoom);
        assert!((expected_px - 1.0).abs() < 1e-12);

        let pts = [[0.0, 0.0], [100.0, 0.0]];
        let color = [1.0, 1.0, 1.0, 1.0];
        let strip = expand_polyline_strip(&pts, width_m, color);
        assert!(strip.len() >= 6);

        // First quad's left/right at start: indices 0,1 of first triangle.
        let left = strip[0].pos;
        let right = strip[1].pos;
        let world_width = (f64::from(left[0]) - f64::from(right[0]))
            .hypot(f64::from(left[1]) - f64::from(right[1]));
        assert!(
            (world_width - width_m).abs() < 1e-5,
            "strip world width {world_width} != {width_m}"
        );
        let screen_px = world_width * 2.0_f64.powf(zoom);
        assert!(
            (screen_px - expected_px).abs() < 1e-6,
            "L9 fail: screen {screen_px} != expected {expected_px}"
        );
    }

    #[test]
    fn casing_is_wider_by_factor() {
        let pts = [[0.0, 0.0], [50.0, 0.0]];
        let (casing, center) = compose_road_segment(&pts, 2.0, "road_paved");
        assert!(!casing.is_empty() && !center.is_empty());
        let c_w = (f64::from(casing[0].pos[1]) - f64::from(casing[1].pos[1])).abs();
        let n_w = (f64::from(center[0].pos[1]) - f64::from(center[1].pos[1])).abs();
        assert!((c_w - 2.0 * ROAD_CASING_FACTOR).abs() < 1e-4);
        assert!((n_w - 2.0).abs() < 1e-4);
    }

    #[test]
    fn dashed_emits_more_than_solid_segment_count() {
        let pts = [[0.0, 0.0], [100.0, 0.0]];
        let color = [1.0, 1.0, 1.0, 1.0];
        let solid = expand_polyline_strip(&pts, 1.0, color);
        let dashed = expand_dashed_polyline_strip(&pts, 1.0, color, 8.0, 6.0);
        assert!(dashed.len() > solid.len());
    }

    #[test]
    fn road_class_gates() {
        assert!(road_class_visible("highway_paved", -2.0));
        assert!(road_class_visible("road_dirt", -2.0));
        assert!(!road_class_visible("road_dirt", -2.1));
        assert!(!road_class_visible("path", 3.9));
        assert!(road_class_visible("path", 4.0));
    }

    /// T-151.4.1: 90° corner produces geometry that covers the outer miter region (no tear).
    #[test]
    fn corner_join_covers_outer_bisector() {
        let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
        let width = 2.0_f64;
        let half = 1.0;
        let strip = expand_polyline_strip(&pts, width, [1.0, 1.0, 1.0, 1.0]);
        assert!(strip.len() > 12, "joined strip should exceed 2 bare quads");
        // Outer corner at (10,0) goes toward SE for a left-hand CCW path that turns left (north).
        // Path goes east then north → left turn; outer is the right side near (10+half, -half area).
        // Sample a point just outside the vertex along the bisector of the exterior.
        // For left turn, exterior is to the right of both segs: sample (10+half*0.7, -half*0.7).
        let sample = [10.0 + half * 0.5, -half * 0.5];
        assert!(
            point_in_any_tri(sample, &strip),
            "outer corner sample {sample:?} not covered — tear/gap at join"
        );
    }

    /// Round cap extends beyond the endpoint by ~half-width.
    #[test]
    fn end_cap_extends_past_endpoint() {
        let pts = [[0.0, 0.0], [10.0, 0.0]];
        let half = 1.0;
        let strip = expand_polyline_strip(&pts, 2.0, [1.0, 1.0, 1.0, 1.0]);
        // Cap at end should cover (10+half*0.7, 0).
        let sample = [10.0 + half * 0.7, 0.0];
        assert!(
            point_in_any_tri(sample, &strip),
            "end cap sample {sample:?} not covered"
        );
    }

    fn point_in_any_tri(p: [f64; 2], strip: &[StripVertex]) -> bool {
        for tri in strip.chunks_exact(3) {
            let a = [f64::from(tri[0].pos[0]), f64::from(tri[0].pos[1])];
            let b = [f64::from(tri[1].pos[0]), f64::from(tri[1].pos[1])];
            let c = [f64::from(tri[2].pos[0]), f64::from(tri[2].pos[1])];
            if point_in_tri(p, a, b, c) {
                return true;
            }
        }
        false
    }

    fn point_in_tri(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
        let s = |u: [f64; 2], v: [f64; 2], w: [f64; 2]| {
            (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])
        };
        let b1 = s(p, a, b) < 0.0;
        let b2 = s(p, b, c) < 0.0;
        let b3 = s(p, c, a) < 0.0;
        (b1 == b2) && (b2 == b3)
    }
}
