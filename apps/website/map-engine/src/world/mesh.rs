//! Role: composing CPU meshes for the static world — contours, forest, land cover.
//! Position: `world` in the map engine.
//! Signals & state: triangle meshes and colour arrays built from static world data.
//! Invariants: **zero GPU.** Nothing here touches a device, a queue or a buffer. That is what
//! makes it cacheable and damage-gateable independently of drawing, which is the point of
//! building it on this side of the packet boundary.
//!
//! T-0xx Phase 2B.1: from `renderers/primitives/compose.rs`, moved whole. It was in the draw
//! path and composed nothing that changes per frame.

/// Ear-clipping triangulation.
// T-0xx Phase 2B.1: re-exported here from `website-graphics-engine`. It stood at
// `renderers/primitives/triangulate`, and its consumers are this file, `world/terrain/roads`
// and the frontend's building viewer — all mesh composition, all beside this module. The
// frontend must not name `website-graphics-engine` itself (gate rule 6), so it needs a path
// in this crate.
pub use website_graphics_engine::draw::triangulate;

use crate::world::environment::vegetation::mass::ForestMassGeometry;
use crate::world::mesh::triangulate::triangulate_region_rings;
use crate::world::mesh::triangulate::triangulate_ring_buffer;

/// Re-export `website_graphics_engine::draw::compose::HairlineGpu`.
// T-0xx Phase 1D: the buffer shapes and the two ring→segment loops moved to
// `website-graphics-engine` (`draw::compose`) and are re-exported here at their former path.
// Four `pub use crate::world::terrain::{roads,water}::mesh::*` re-exports were DELETED rather than
// moved — they were a shortcut that let a caller reach road and sea meshing through the
// renderer, which is the exact coupling the split exists to remove. Their callers name
// `crate::world::terrain::…` directly now.
pub use website_graphics_engine::draw::compose::HairlineGpu;

/// Re-export `website_graphics_engine::draw::compose::PolyMeshGpu`.
pub use website_graphics_engine::draw::compose::PolyMeshGpu;

/// Re-export `website_graphics_engine::draw::compose::mesh_from_tri`.
pub use website_graphics_engine::draw::compose::mesh_from_tri;

/// Re-export `website_graphics_engine::draw::compose::retint_fill_alpha`.
pub use website_graphics_engine::draw::compose::retint_fill_alpha;

use website_graphics_engine::draw::compose::u8_rgba_to_f32;

/// Contour interleaved `[x0,y0,x1,y1]…` → hairline verts with fixed rgba.
#[must_use]
pub fn compose_contour_hairlines(segments: &[f32], rgba: [u8; 4]) -> HairlineGpu {
    website_graphics_engine::draw::compose::compose_hairlines(segments, rgba)
}

/// Ownership: contours are the ONLY caller (a two-tone set); [`compose_contour_hairlines`] stays the single-colour path for the forest outline (`forest_mass.rs`), so this is an additive signature — the flat-`Vec<f32>` compose is unchanged.
///
/// The adapter half of the split: a `ContourRing` becomes a bare point slice plus its `closed`
/// flag, which is the only column the renderer's edge loop reads. Its `level` stays here —
/// it decided which rings exist and which are summits before this call, and the renderer has
/// no business re-deriving that.
#[must_use]
pub fn compose_two_tone_contours(
    rings: &[crate::world::terrain::relief::contours::ContourRing],
    summit_idx: &[usize],
    base_rgba: [u8; 4],
    summit_rgba: [u8; 4],
) -> HairlineGpu {
    let points: Vec<&[(f64, f64)]> = rings.iter().map(|r| r.points.as_slice()).collect();
    let closed: Vec<bool> = rings.iter().map(|r| r.closed).collect();
    website_graphics_engine::draw::compose::compose_two_tone_hairlines(
        &points,
        &closed,
        summit_idx,
        base_rgba,
        summit_rgba,
    )
}

/// Contour stroke colour — `contourLayer.ts` `CONTOUR_RGBA`.
pub const CONTOUR_RGBA: [u8; 4] = [120, 96, 64, 200];

/// Forest outline — `forestMassLayer.ts` `FOREST_OUTLINE_RGBA`.
pub const FOREST_OUTLINE_RGBA: [u8; 4] = [24, 90, 45, 230];

/// Forest fill RGB — `forestMass.ts` `FOREST_FILL_RGB`.
pub const FOREST_FILL_RGB: [u8; 3] = [34, 120, 60];

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

/// Forest mass → fill mesh + outline hairlines.
#[must_use]
pub fn compose_forest_mesh(
    geo: &ForestMassGeometry,
    fill_alpha: f64,
) -> (PolyMeshGpu, HairlineGpu) {
    let fill = if geo.fill_positions.is_empty() || fill_alpha <= 0.0 {
        PolyMeshGpu::default()
    } else {
        let (mesh, _) = triangulate_ring_buffer(&geo.fill_positions, &geo.fill_start_indices, None);
        let n_verts = mesh.positions.len() / 2;
        let rgba = [
            FOREST_FILL_RGB[0],
            FOREST_FILL_RGB[1],
            FOREST_FILL_RGB[2],
            (255.0 * fill_alpha).round().clamp(0.0, 255.0) as u8,
        ];
        let mut cols = Vec::with_capacity(n_verts * 4);
        for _ in 0..n_verts {
            cols.extend_from_slice(&rgba);
        }
        mesh_from_tri(mesh, &cols, 1.0)
    };
    let outline = compose_contour_hairlines(&geo.outline_segments, FOREST_OUTLINE_RGBA);
    (fill, outline)
}

#[cfg(test)]
#[path = "tests/mesh_tests.rs"]
mod tests;
