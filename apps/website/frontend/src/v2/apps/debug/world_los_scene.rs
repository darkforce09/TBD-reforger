//! Plan geometry for the world-occluder bench at `/debug/world-los`.
//!
//! **Role:** turns the catalogue of objects placed around a map point, plus one A → B line-of-sight
//! probe, into packed vertex lanes — buildings on the slab lane with their eye-height section cuts
//! on the wall lane, props, vehicles and water on the furniture lane, trees and rocks on the
//! vegetation lane, proxy boxes as amber outlines on the portals-outline lane, and the probe ray on
//! its own lane coloured by what it crossed.
//! **Position:** the pure half of the bench, below [`super::world_los`], which owns the wasm host,
//! the fetches and the canvas. It shares the architectural lane set with
//! [`super::building_interior`].
//! **Signals & state:** none. Every function here is a pure map-frame → vertex-buffer transform,
//! so the whole module is exercised by the native test suite.
//! **Invariants:** coordinates are the map frame (`x` east, `y` north) in metres; colours are
//! straight `[r, g, b, a]` in linear space; a lane is appended to, never cleared, so a caller
//! composes several producers into one [`InteriorLanes`].

use website_map_engine::world::architecture::blueprint::attribution_1::LosHit;
use website_map_engine::world::architecture::blueprint::attribution_1::LosHitKind;
use website_map_engine::world::terrain::roads::styling::expand_polyline_strip;

use super::building_interior::{InteriorLanes, RAY_FOLIAGE};

/// Fill of a building footprint.
pub const COL_BUILDING: [f32; 4] = [0.62, 0.66, 0.74, 0.50];
/// Outline of a building's eye-height section cut, drawn over its fill.
pub const COL_BUILDING_CUT: [f32; 4] = [0.92, 0.94, 0.98, 1.0];
/// Fill of a prop footprint, and of any kind the catalogue does not name.
pub const COL_PROP: [f32; 4] = [0.50, 0.55, 0.62, 0.55];
/// Fill of a vehicle footprint.
pub const COL_VEHICLE: [f32; 4] = [0.72, 0.56, 0.30, 0.65];
/// Fill of a water-body footprint.
pub const COL_WATER: [f32; 4] = [0.25, 0.45, 0.70, 0.45];
/// Fill of a rock footprint.
pub const COL_ROCK: [f32; 4] = [0.46, 0.43, 0.40, 0.60];
/// Fill of a tree footprint.
pub const COL_TREE: [f32; 4] = [0.30, 0.55, 0.28, 0.45];
/// Outline drawn around a tree footprint so canopies stay separable where they overlap.
pub const COL_TREE_EDGE: [f32; 4] = [0.45, 0.75, 0.40, 0.80];
/// Outline of an object still drawn as a proxy box, its real geometry not yet loaded.
pub const COL_PROXY: [f32; 4] = [0.95, 0.70, 0.20, 0.95];
/// Probe-ray colour over a span that nothing occludes.
pub const RAY_CLEAR: [f32; 4] = [0.25, 0.90, 0.40, 1.0];
/// Probe-ray colour over a span crossing glazing, which degrades rather than blocks sight.
pub const RAY_GLASS: [f32; 4] = [0.20, 0.80, 0.95, 1.0];
/// Probe-ray colour over a span an opaque surface blocks.
pub const RAY_BLOCKED: [f32; 4] = [0.95, 0.25, 0.20, 1.0];
/// Probe-ray colour over a span whose verdict is still provisional because a proxy stands in it.
pub const RAY_PROVISIONAL: [f32; 4] = [0.95, 0.70, 0.20, 1.0];
/// Strip widths in world metres (a village-scale bench, not a room).
pub const RAY_WIDTH_M: f64 = 0.6;
/// Width in world metres of a building's section-cut outline strip.
pub const CUT_WIDTH_M: f64 = 0.25;

/// One placed object's plan rectangle (map frame `x`, `y_north`).
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    /// Catalogue placement id the rectangle came from.
    pub pid: u16,
    /// Catalogue kind (`building`, `vehicle`, `water`, `rock`, `tree`, otherwise a prop) that
    /// selects the fill colour and the lane.
    pub kind: String,
    /// South-west corner in map metres.
    pub min: [f64; 2],
    /// North-east corner in map metres.
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
#[path = "tests/world_los_scene/scene_primitives.rs"]
mod scene_primitives_tests;
