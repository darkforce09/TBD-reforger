//! **Role:** composing CPU meshes for the static world — contours, forest, land cover.
//! **Position:** `mesh_composition` in `map_asset_loading`; the mesh shapes, the ring
//! triangulation and the ring→segment loops come from `render_primitives::draw`; the world,
//! forest and relief loaders compose here and upload the result through the asset sink.
//! **Signals & state:** none; triangle meshes and colour arrays built from static world data.
//! **Invariants:** **zero GPU.** Nothing here touches a device, a queue or a buffer. That is what
//! makes it cacheable and damage-gateable independently of drawing, which is the point of
//! building it on this side of the packet boundary.

use render_primitives::color_normalization::u8_rgba_to_f32;
use render_primitives::draw::compose::{HairlineGpu, PolyMeshGpu};
use render_primitives::draw::triangulate::triangulate_region_rings;

/// Contour interleaved `[x0,y0,x1,y1]…` → hairline verts with fixed rgba.
#[must_use]
pub fn compose_contour_hairlines(segments: &[f32], rgba: [u8; 4]) -> HairlineGpu {
    render_primitives::draw::compose::compose_hairlines(segments, rgba)
}

/// Ownership: contours are the ONLY caller (a two-tone set); [`compose_contour_hairlines`] stays the single-colour path for the forest outline (`forest_mass.rs`), so this is an additive signature — the flat-`Vec<f32>` compose is unchanged.
///
/// The adapter half of the split: a `ContourRing` becomes a bare point slice plus its `closed`
/// flag, which is the only column the renderer's edge loop reads. Its `level` stays here —
/// it decided which rings exist and which are summits before this call, and the renderer has
/// no business re-deriving that.
#[must_use]
pub fn compose_two_tone_contours(
    rings: &[terrain_relief::contours::ContourRing],
    summit_idx: &[usize],
    base_rgba: [u8; 4],
    summit_rgba: [u8; 4],
) -> HairlineGpu {
    let points: Vec<&[(f64, f64)]> = rings.iter().map(|r| r.points.as_slice()).collect();
    let closed: Vec<bool> = rings.iter().map(|r| r.closed).collect();
    render_primitives::draw::compose::compose_two_tone_hairlines(
        &points,
        &closed,
        summit_idx,
        base_rgba,
        summit_rgba,
    )
}

/// Forest outline — `forestMassLayer.ts` `FOREST_OUTLINE_RGBA`.
pub const FOREST_OUTLINE_RGBA: [u8; 4] = [24, 90, 45, 230];

/// Land-cover fill colours by kind — `landCoverRegions.ts` `LANDCOVER_FILL`.
#[must_use]
pub fn landcover_fill(kind: &str) -> [u8; 4] {
    match kind {
        "forest" => [46, 90, 50, 38],
        "field" => [205, 198, 163, 31],
        "waterBody" => [90, 140, 185, 89],
        _ => [128, 128, 128, 38],
    }
}

/// One land-cover region for compose (mirrors `LandCoverRegion` without serde).
pub struct LandcoverInput<'a> {
    /// Kind.
    pub kind: &'a str,

    /// Rings.
    pub rings: &'a [Vec<[f64; 2]>],
}

/// Compose all land-cover regions into one polygon mesh.
#[must_use]
pub fn compose_landcover_mesh(regions: &[LandcoverInput<'_>]) -> PolyMeshGpu {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut base = 0_u32;
    let mut polygon_count = 0_u32;

    for r in regions {
        if r.rings.is_empty() {
            continue;
        }
        let mesh = triangulate_region_rings(r.rings);
        if mesh.indices.is_empty() {
            continue;
        }
        let rgba = landcover_fill(r.kind);
        let c = u8_rgba_to_f32(rgba, 1.0);
        let n_verts = mesh.positions.len() / 2;
        positions.extend_from_slice(&mesh.positions);
        for _ in 0..n_verts {
            colors.extend_from_slice(&c);
        }
        for &ix in &mesh.indices {
            indices.push(base + ix);
        }
        base += n_verts as u32;
        polygon_count += 1;
    }

    PolyMeshGpu {
        positions,
        colors,
        indices,
        polygon_count,
    }
}

#[cfg(test)]
#[path = "tests/mesh_composition_tests.rs"]
mod tests;
