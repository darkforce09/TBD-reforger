//! Role: compose.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::vegetation::mass::ForestMassGeometry;
use crate::renderers::primitives::triangulate::TriMesh;
use crate::renderers::primitives::triangulate::triangulate_region_rings;
use crate::renderers::primitives::triangulate::triangulate_ring_buffer;

/// Re-export `crate::terrain::roads::mesh::RoadInput`.
pub use crate::terrain::roads::mesh::RoadInput;

/// Re-export `crate::terrain::roads::mesh::RoadMeshGpu`.
pub use crate::terrain::roads::mesh::RoadMeshGpu;

/// Re-export `crate::terrain::roads::mesh::compose_roads_mesh`.
pub use crate::terrain::roads::mesh::compose_roads_mesh;

/// Re-export `crate::terrain::water::mesh::compose_sea_mesh`.
pub use crate::terrain::water::mesh::compose_sea_mesh;

/// Packed polygon fill for a `PolygonFill` lane.
#[derive(Clone, Debug, Default)]
pub struct PolyMeshGpu {
    /// Positions.
    pub positions: Vec<f32>,

    /// Colors.
    pub colors: Vec<f32>,

    /// Indices.
    pub indices: Vec<u32>,

    /// Polygon count.
    pub polygon_count: u32,
}

/// Hairline segment list for a `Polyline` LineList lane.
#[derive(Clone, Debug, Default)]
pub struct HairlineGpu {
    /// Flat `[x,y,r,g,b,a]…` — 2 verts per segment.
    pub verts: Vec<f32>,

    /// Segment count.
    pub segment_count: u32,
}

fn u8_rgba_to_f32(c: [u8; 4], layer_alpha: f32) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        (f32::from(c[3]) / 255.0) * layer_alpha,
    ]
}

/// Retint fill alpha.
pub fn retint_fill_alpha(colors: &mut [f32], alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    for c in colors.chunks_exact_mut(4) {
        c[3] = a;
    }
}

/// Mesh from tri.
pub(crate) fn mesh_from_tri(mesh: TriMesh, colors_u8: &[u8], layer_alpha: f32) -> PolyMeshGpu {
    if mesh.indices.is_empty() {
        return PolyMeshGpu::default();
    }
    let n_verts = mesh.positions.len() / 2;
    let mut colors = Vec::with_capacity(n_verts * 4);
    for vi in 0..n_verts {
        let ci = vi * 4;
        let rgba = if ci + 3 < colors_u8.len() {
            [
                colors_u8[ci],
                colors_u8[ci + 1],
                colors_u8[ci + 2],
                colors_u8[ci + 3],
            ]
        } else {
            [255, 255, 255, 255]
        };
        let c = u8_rgba_to_f32(rgba, layer_alpha);
        colors.extend_from_slice(&c);
    }

    #[allow(clippy::cast_possible_truncation)]
    let polygon_count = (mesh.indices.len() / 3) as u32;
    PolyMeshGpu {
        positions: mesh.positions,
        colors,
        indices: mesh.indices,
        polygon_count,
    }
}

/// Contour interleaved `[x0,y0,x1,y1]…` → hairline verts with fixed rgba.
#[must_use]
pub fn compose_contour_hairlines(segments: &[f32], rgba: [u8; 4]) -> HairlineGpu {
    if segments.len() < 4 {
        return HairlineGpu::default();
    }
    let c = u8_rgba_to_f32(rgba, 1.0);
    let mut verts = Vec::with_capacity(segments.len() / 4 * 12);
    let mut segment_count = 0_u32;
    for seg in segments.chunks_exact(4) {
        for (x, y) in [(seg[0], seg[1]), (seg[2], seg[3])] {
            verts.push(x);
            verts.push(y);
            verts.extend_from_slice(&c);
        }
        segment_count += 1;
    }
    HairlineGpu {
        verts,
        segment_count,
    }
}

/// Ownership: contours are the ONLY caller (a two-tone set); [`compose_contour_hairlines`] stays the single-colour path for the forest outline (`forest_mass.rs`), so this is an additive signature — the flat-`Vec<f32>` compose is unchanged.
#[must_use]
pub fn compose_two_tone_contours(
    rings: &[crate::terrain::relief::contours::ContourRing],
    summit_idx: &[usize],
    base_rgba: [u8; 4],
    summit_rgba: [u8; 4],
) -> HairlineGpu {
    let base = u8_rgba_to_f32(base_rgba, 1.0);
    let summit = u8_rgba_to_f32(summit_rgba, 1.0);
    let mut verts: Vec<f32> = Vec::new();
    let mut segment_count = 0_u32;
    for (i, ring) in rings.iter().enumerate() {
        if ring.points.len() < 2 {
            continue;
        }
        let c = if summit_idx.contains(&i) {
            &summit
        } else {
            &base
        };
        let n = ring.points.len();

        let edges = if ring.closed { n } else { n - 1 };
        for e in 0..edges {
            let a = ring.points[e];
            let b = ring.points[(e + 1) % n];
            for (x, y) in [a, b] {
                verts.push(x as f32);
                verts.push(y as f32);
                verts.extend_from_slice(c);
            }
            segment_count += 1;
        }
    }
    HairlineGpu {
        verts,
        segment_count,
    }
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
#[path = "tests/compose_tests.rs"]
mod tests;
