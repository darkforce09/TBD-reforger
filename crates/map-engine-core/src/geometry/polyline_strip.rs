//! Meter-width polyline strip expansion for road casing/centerline (T-151.4 L2/L9).
//!
//! Deck `PathLayer` draws strokes in world meters (`widthUnits: 'meters'`). On wgpu/WebGL2
//! native line width is 1 px, so roads are expanded on the CPU into a triangle-list strip
//! (two tris per segment). Hairline layers (grid, contours, forest outline) stay on `LineList`.
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
/// Deck ortho: world meters × `2^zoom` = CSS pixels at the reference (no DPR here — pure world).
#[must_use]
pub fn projected_width_px(width_m: f64, deck_zoom: f64) -> f64 {
    width_m * 2.0_f64.powf(deck_zoom)
}

/// Expand a polyline centerline into a triangle-list strip of half-width `width_m / 2`.
/// Per-segment perpendicular offset (no miter joins) — sufficient for L9 midpoint exactness
/// on straight segments and good enough for road visual parity.
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
    let mut out = Vec::with_capacity((points.len() - 1) * 6);

    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 1e-12 {
            continue;
        }
        // Left-hand perpendicular unit (CCW of direction).
        let nx = -dy / len * half;
        let ny = dx / len * half;

        let a_l = [a[0] + nx, a[1] + ny];
        let a_r = [a[0] - nx, a[1] - ny];
        let b_l = [b[0] + nx, b[1] + ny];
        let b_r = [b[0] - nx, b[1] - ny];

        // Two triangles: a_l–a_r–b_r and a_l–b_r–b_l.
        for p in [a_l, a_r, b_r, a_l, b_r, b_l] {
            out.push(StripVertex {
                pos: [p[0] as f32, p[1] as f32],
                color,
            });
        }
    }
    out
}

/// Dash pattern [dash, gap] in meters (mirror `dashArrayFor`: dashed roads = [8, 6]).
/// Emits only the dash intervals of the path as separate strip segments.
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

    // Flatten path into cumulative arc-length samples, then walk dash/gap.
    let mut out = Vec::new();
    let mut cursor = 0.0_f64; // distance along path of current sample start
    let mut in_dash = true;
    let mut phase = 0.0_f64; // distance into current dash/gap phase

    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let seg_len = (b[0] - a[0]).hypot(b[1] - a[1]);
        if seg_len < 1e-12 {
            continue;
        }
        let dir = [(b[0] - a[0]) / seg_len, (b[1] - a[1]) / seg_len];
        let mut t = 0.0_f64; // distance into this segment
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
            cursor += step;
            let _ = cursor; // silence; kept for future debug
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
        let expected_px = projected_width_px(width_m, zoom); // 4 * 0.25 = 1.0
        assert!((expected_px - 1.0).abs() < 1e-12);

        // Geometric strip half-width in world meters is width_m/2; at midpoint the full
        // cross-section span is width_m. Screen span = width_m * 2^zoom.
        let pts = [[0.0, 0.0], [100.0, 0.0]];
        let color = [1.0, 1.0, 1.0, 1.0];
        let strip = expand_polyline_strip(&pts, width_m, color);
        assert_eq!(strip.len(), 6); // one segment → 2 tris → 6 verts

        // Midpoint cross-section: take first triangle's two left/right verts at a (indices 0,1).
        let left = strip[0].pos;
        let right = strip[1].pos;
        let world_width = (f64::from(left[0]) - f64::from(right[0]))
            .hypot(f64::from(left[1]) - f64::from(right[1]));
        assert!(
            (world_width - width_m).abs() < 1e-6,
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
        // Casing half-span = 2*1.4/2 = 1.4; center = 1.0
        let c_w = (f64::from(casing[0].pos[1]) - f64::from(casing[1].pos[1])).abs();
        let n_w = (f64::from(center[0].pos[1]) - f64::from(center[1].pos[1])).abs();
        assert!((c_w - 2.0 * ROAD_CASING_FACTOR).abs() < 1e-5);
        assert!((n_w - 2.0).abs() < 1e-5);
    }

    #[test]
    fn dashed_emits_less_than_solid() {
        let pts = [[0.0, 0.0], [100.0, 0.0]];
        let color = [1.0, 1.0, 1.0, 1.0];
        let solid = expand_polyline_strip(&pts, 1.0, color);
        let dashed = expand_dashed_polyline_strip(&pts, 1.0, color, 8.0, 6.0);
        // 100 m path: dash 8 + gap 6 → ~7 dashes, each a segment; solid is one segment.
        assert!(dashed.len() > solid.len());
        assert_eq!(solid.len(), 6);
    }

    #[test]
    fn road_class_gates() {
        assert!(road_class_visible("highway_paved", -2.0));
        assert!(road_class_visible("road_dirt", -2.0));
        assert!(!road_class_visible("road_dirt", -2.1));
        assert!(!road_class_visible("path", 3.9));
        assert!(road_class_visible("path", 4.0));
    }
}
