//! T-090.12.5 — the `/debug/world-los` bench's pure geometry (native-tested): placed objects as
//! plan footprints on the T-090.11.6 architectural lanes (buildings on the slab lane with their
//! eye-height section cuts on the wall lane, props / vehicles / water on the furniture lane,
//! trees and rocks on the vegetation lane, proxies as amber outlines on the portals-outline
//! lane) and the A → B ray on the probe lane coloured by what it crossed.

use map_engine_core::building_blueprint::{LosHit, LosHitKind};
use map_engine_core::geometry::polyline_strip::expand_polyline_strip;

use super::building_interior::{InteriorLanes, RAY_FOLIAGE};

pub const COL_BUILDING: [f32; 4] = [0.62, 0.66, 0.74, 0.50];
pub const COL_BUILDING_CUT: [f32; 4] = [0.92, 0.94, 0.98, 1.0];
pub const COL_PROP: [f32; 4] = [0.50, 0.55, 0.62, 0.55];
pub const COL_VEHICLE: [f32; 4] = [0.72, 0.56, 0.30, 0.65];
pub const COL_WATER: [f32; 4] = [0.25, 0.45, 0.70, 0.45];
pub const COL_ROCK: [f32; 4] = [0.46, 0.43, 0.40, 0.60];
pub const COL_TREE: [f32; 4] = [0.30, 0.55, 0.28, 0.45];
pub const COL_TREE_EDGE: [f32; 4] = [0.45, 0.75, 0.40, 0.80];
pub const COL_PROXY: [f32; 4] = [0.95, 0.70, 0.20, 0.95];
pub const RAY_CLEAR: [f32; 4] = [0.25, 0.90, 0.40, 1.0];
pub const RAY_GLASS: [f32; 4] = [0.20, 0.80, 0.95, 1.0];
pub const RAY_BLOCKED: [f32; 4] = [0.95, 0.25, 0.20, 1.0];
pub const RAY_PROVISIONAL: [f32; 4] = [0.95, 0.70, 0.20, 1.0];
/// Strip widths in world metres (a village-scale bench, not a room).
pub const RAY_WIDTH_M: f64 = 0.6;
pub const CUT_WIDTH_M: f64 = 0.25;

/// One placed object's plan rectangle (map frame `x`, `y_north`).
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    pub pid: u16,
    pub kind: String,
    pub min: [f64; 2],
    pub max: [f64; 2],
    /// Still a proxy box (descriptor / BLAS not loaded).
    pub proxy: bool,
}

/// Fill colour per catalogue kind.
#[must_use]
pub fn kind_color(kind: &str) -> [f32; 4] {
    match kind {
        "building" => COL_BUILDING,
        "vehicle" => COL_VEHICLE,
        "water" => COL_WATER,
        "rock" => COL_ROCK,
        "tree" => COL_TREE,
        _ => COL_PROP,
    }
}

fn quad(
    pos: &mut Vec<f32>,
    col: &mut Vec<f32>,
    idx: &mut Vec<u32>,
    min: [f64; 2],
    max: [f64; 2],
    c: [f32; 4],
) {
    #[allow(clippy::cast_possible_truncation)]
    let base = (pos.len() / 2) as u32;
    for p in [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ] {
        pos.extend_from_slice(&[p[0] as f32, p[1] as f32]);
        col.extend_from_slice(&c);
    }
    idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn seg(out: &mut Vec<f32>, a: [f64; 2], b: [f64; 2], c: [f32; 4]) {
    for p in [a, b] {
        out.extend_from_slice(&[p[0] as f32, p[1] as f32, c[0], c[1], c[2], c[3]]);
    }
}

fn rect_outline(out: &mut Vec<f32>, min: [f64; 2], max: [f64; 2], c: [f32; 4]) -> u32 {
    let corners = [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ];
    for i in 0..4 {
        seg(out, corners[i], corners[(i + 1) % 4], c);
    }
    4
}

fn strip(out: &mut Vec<f32>, a: [f64; 2], b: [f64; 2], width: f64, c: [f32; 4]) -> u32 {
    let verts = expand_polyline_strip(&[a, b], width, c);
    for v in &verts {
        out.extend_from_slice(&[
            v.pos[0], v.pos[1], v.color[0], v.color[1], v.color[2], v.color[3],
        ]);
    }
    1
}

/// The static lanes: footprints by kind, building section cuts (`cuts`, world plan segments),
/// proxies as amber outlines.
#[must_use]
pub fn build_bench_lanes(fps: &[Footprint], cuts: &[[[f64; 2]; 2]]) -> InteriorLanes {
    let mut l = InteriorLanes::default();
    for f in fps {
        if f.proxy {
            l.portals_outline_count +=
                rect_outline(&mut l.portals_outline, f.min, f.max, COL_PROXY);
            continue;
        }
        let c = kind_color(&f.kind);
        match f.kind.as_str() {
            "building" => quad(
                &mut l.slabs_pos,
                &mut l.slabs_col,
                &mut l.slabs_idx,
                f.min,
                f.max,
                c,
            ),
            "tree" => {
                quad(
                    &mut l.vegetation_pos,
                    &mut l.vegetation_col,
                    &mut l.vegetation_idx,
                    f.min,
                    f.max,
                    c,
                );
                l.vegetation_outline_count +=
                    rect_outline(&mut l.vegetation_outline, f.min, f.max, COL_TREE_EDGE);
                l.tree_count += 1;
            }
            "rock" => quad(
                &mut l.vegetation_pos,
                &mut l.vegetation_col,
                &mut l.vegetation_idx,
                f.min,
                f.max,
                c,
            ),
            _ => {
                quad(
                    &mut l.furniture_pos,
                    &mut l.furniture_col,
                    &mut l.furniture_idx,
                    f.min,
                    f.max,
                    c,
                );
                l.furniture_count += 1;
            }
        }
    }
    for s in cuts {
        l.wall_count += strip(&mut l.walls, s[0], s[1], CUT_WIDTH_M, COL_BUILDING_CUT);
    }
    l
}

/// The colour the ray takes AFTER crossing `h`.
fn colour_after(h: &LosHit, before: [f32; 4]) -> [f32; 4] {
    match h.kind {
        LosHitKind::Glass | LosHitKind::Window if h.concealment < 1.0 => RAY_GLASS,
        LosHitKind::Foliage => RAY_FOLIAGE,
        LosHitKind::DoorOpen | LosHitKind::DoorAperture => before,
        _ if h.concealment >= 1.0 => RAY_BLOCKED,
        _ => before,
    }
}

/// The A → B probe strip: spans between crossings coloured by the material crossed; the final
/// span red when blocked, amber when a proxy decided it.
#[must_use]
pub fn ray_strip(
    obs: [f64; 2],
    tgt: [f64; 2],
    hits: &[LosHit],
    is_clear: bool,
    provisional: bool,
) -> (Vec<f32>, u32) {
    let at = |t: f64| {
        [
            obs[0] + t * (tgt[0] - obs[0]),
            obs[1] + t * (tgt[1] - obs[1]),
        ]
    };
    let mut out = Vec::new();
    let mut count = 0u32;
    let mut colour = RAY_CLEAR;
    let mut t_prev = 0.0f64;
    let mut ended = false;
    for h in hits {
        let t = h.t.clamp(0.0, 1.0);
        if t - t_prev > 1e-6 {
            count += strip(&mut out, at(t_prev), at(t), RAY_WIDTH_M, colour);
        }
        t_prev = t;
        colour = colour_after(h, colour);
        if colour == RAY_BLOCKED {
            ended = true;
            break;
        }
    }
    let tail = if provisional {
        RAY_PROVISIONAL
    } else if !is_clear || ended {
        RAY_BLOCKED
    } else {
        colour
    };
    if 1.0 - t_prev > 1e-6 {
        count += strip(&mut out, at(t_prev), at(1.0), RAY_WIDTH_M, tail);
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(pid: u16, kind: &str, x: f64, proxy: bool) -> Footprint {
        Footprint {
            pid,
            kind: kind.into(),
            min: [x, 0.0],
            max: [x + 2.0, 2.0],
            proxy,
        }
    }

    fn hit(t: f64, kind: LosHitKind, c: f64) -> LosHit {
        LosHit {
            t,
            pos: [0.0; 3],
            kind,
            id: "x".into(),
            concealment: c,
        }
    }

    #[test]
    fn footprints_route_by_kind_and_proxies_are_amber_outlines() {
        let l = build_bench_lanes(
            &[
                fp(1, "building", 0.0, false),
                fp(2, "tree", 10.0, false),
                fp(3, "rock", 20.0, false),
                fp(4, "prop", 30.0, false),
                fp(5, "vehicle", 40.0, false),
                fp(6, "building", 50.0, true),
            ],
            &[[[0.0, 0.0], [2.0, 0.0]]],
        );
        assert_eq!(l.slabs_idx.len(), 6, "one building quad");
        assert_eq!(l.vegetation_idx.len(), 12, "tree + rock quads");
        assert_eq!(l.furniture_idx.len(), 12, "prop + vehicle quads");
        assert_eq!(l.furniture_count, 2);
        assert_eq!(l.tree_count, 1);
        assert_eq!(l.vegetation_outline_count, 4, "the tree's rim");
        assert_eq!(l.portals_outline_count, 4, "the proxy's amber outline");
        assert_eq!(l.wall_count, 1, "one section cut strip");
        assert_eq!(l.walls.len() % 6, 0);
        assert_eq!(kind_color("water"), COL_WATER);
        assert_eq!(kind_color("anything"), COL_PROP);
    }

    #[test]
    fn ray_spans_follow_the_crossings() {
        let (packed, n) = ray_strip([0.0, 0.0], [100.0, 0.0], &[], true, false);
        assert_eq!(n, 1);
        assert_eq!(packed[2..6], RAY_CLEAR);
        // Blocked halfway: green then red, nothing after the terminal hit is recoloured.
        let (packed, n) = ray_strip(
            [0.0, 0.0],
            [100.0, 0.0],
            &[
                hit(0.5, LosHitKind::Solid, 1.0),
                hit(0.8, LosHitKind::Glass, 0.05),
            ],
            false,
            false,
        );
        assert_eq!(n, 2);
        let stride = packed.len() / 2;
        assert_eq!(packed[2..6], RAY_CLEAR);
        assert_eq!(packed[stride + 2..stride + 6], RAY_BLOCKED);
        // Glass then foliage, clear: three spans, cyan then yellow-green.
        let (packed, n) = ray_strip(
            [0.0, 0.0],
            [100.0, 0.0],
            &[
                hit(0.2, LosHitKind::Glass, 0.05),
                hit(0.6, LosHitKind::Foliage, 0.3),
            ],
            true,
            false,
        );
        assert_eq!(n, 3);
        let stride = packed.len() / 3;
        assert_eq!(packed[stride + 2..stride + 6], RAY_GLASS);
        assert_eq!(packed[2 * stride + 2..2 * stride + 6], RAY_FOLIAGE);
        // Provisional: the tail is amber.
        let (packed, _) = ray_strip([0.0, 0.0], [100.0, 0.0], &[], false, true);
        assert_eq!(packed[2..6], RAY_PROVISIONAL);
    }
}
