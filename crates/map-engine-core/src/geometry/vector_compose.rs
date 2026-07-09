//! Compose GPU-ready vector meshes from Class R geometry (T-151.4 L4–L7).
//!
//! Output packing (world meters; engine subtracts ANCHOR on upload):
//! - Polygon mesh: `positions [x,y]…`, `colors [r,g,b,a] f32…`, `indices u32…`
//! - Hairline LineList: `[x,y,r,g,b,a]…` (6 f32/vert, 2 verts/segment)
//! - Wide strip: same 6-f32 packing via [`polyline_strip::pack_strip_verts`]

use super::forest_mass::ForestMassGeometry;
use super::polyline_strip::{
    StripVertex, compose_road_segment, pack_strip_verts, road_class_visible,
};
use super::sea_band::SeaBandGeometry;
use super::triangulate::{TriMesh, triangulate_region_rings, triangulate_ring_buffer};

/// Packed polygon fill for a `PolygonFill` lane.
#[derive(Clone, Debug, Default)]
pub struct PolyMeshGpu {
    pub positions: Vec<f32>,
    pub colors: Vec<f32>,
    pub indices: Vec<u32>,
    pub polygon_count: u32,
}

/// Hairline segment list for a `Polyline` LineList lane.
#[derive(Clone, Debug, Default)]
pub struct HairlineGpu {
    /// Flat `[x,y,r,g,b,a]…` — 2 verts per segment.
    pub verts: Vec<f32>,
    pub segment_count: u32,
}

/// Road strip pair (casing under, centerline over).
#[derive(Clone, Debug, Default)]
pub struct RoadMeshGpu {
    pub casing: Vec<f32>,
    pub centerline: Vec<f32>,
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

fn mesh_from_tri(mesh: TriMesh, colors_u8: &[u8], layer_alpha: f32) -> PolyMeshGpu {
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
    // polygon_count ≈ triangle count for stats (coarse).
    #[allow(clippy::cast_possible_truncation)]
    let polygon_count = (mesh.indices.len() / 3) as u32;
    PolyMeshGpu {
        positions: mesh.positions,
        colors,
        indices: mesh.indices,
        polygon_count,
    }
}

/// Sea-band geometry → triangulated fill mesh with `layer_alpha` (seaFillAlpha).
#[must_use]
pub fn compose_sea_mesh(geo: &SeaBandGeometry, layer_alpha: f64) -> PolyMeshGpu {
    if geo.polygon_count == 0 || layer_alpha <= 0.0 {
        return PolyMeshGpu::default();
    }
    let (mesh, cols) = triangulate_ring_buffer(
        &geo.fill_positions,
        &geo.fill_start_indices,
        Some(&geo.fill_colors),
    );
    mesh_from_tri(mesh, &cols, layer_alpha as f32)
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
    pub kind: &'a str,
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

/// Road segment input for compose.
pub struct RoadInput<'a> {
    pub road_class: &'a str,
    pub points: &'a [[f64; 2]],
    pub width_m: f64,
}

/// Compose visible road casing + centerline strips at `deck_zoom`.
#[must_use]
pub fn compose_roads_mesh(roads: &[RoadInput<'_>], deck_zoom: f64) -> RoadMeshGpu {
    let mut casing_v: Vec<StripVertex> = Vec::new();
    let mut center_v: Vec<StripVertex> = Vec::new();
    let mut segment_count = 0_u32;
    for r in roads {
        if !road_class_visible(r.road_class, deck_zoom) {
            continue;
        }
        let (c, n) = compose_road_segment(r.points, r.width_m, r.road_class);
        if c.is_empty() && n.is_empty() {
            continue;
        }
        casing_v.extend(c);
        center_v.extend(n);
        segment_count += 1;
    }
    RoadMeshGpu {
        casing: pack_strip_verts(&casing_v),
        centerline: pack_strip_verts(&center_v),
        segment_count,
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

/// Axis-aligned marquee rect → one quad (two tris).
#[must_use]
pub fn compose_marquee_mesh(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> PolyMeshGpu {
    // Aegis selection tint (semi-transparent primary).
    let c = u8_rgba_to_f32([173, 198, 255, 60], 1.0);
    let positions = [
        min_x as f32,
        min_y as f32,
        max_x as f32,
        min_y as f32,
        max_x as f32,
        max_y as f32,
        min_x as f32,
        max_y as f32,
    ];
    let mut colors = Vec::with_capacity(16);
    for _ in 0..4 {
        colors.extend_from_slice(&c);
    }
    PolyMeshGpu {
        positions: positions.to_vec(),
        colors,
        indices: vec![0, 1, 2, 0, 2, 3],
        polygon_count: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dem::DemVectorGrid;
    use crate::geometry::sea_band::build_sea_band_geometry;

    #[test]
    fn marquee_is_two_tris() {
        let m = compose_marquee_mesh(0.0, 0.0, 10.0, 20.0);
        assert_eq!(m.indices.len(), 6);
        assert_eq!(m.polygon_count, 1);
    }

    #[test]
    fn roads_gated_by_zoom() {
        let pts = [[0.0, 0.0], [100.0, 0.0]];
        let roads = [RoadInput {
            road_class: "path",
            points: &pts,
            width_m: 1.0,
        }];
        let hidden = compose_roads_mesh(&roads, 3.0);
        assert_eq!(hidden.segment_count, 0);
        let shown = compose_roads_mesh(&roads, 4.0);
        assert_eq!(shown.segment_count, 1);
        assert!(!shown.centerline.is_empty());
    }

    #[test]
    fn sea_compose_from_empty_is_empty() {
        let g = SeaBandGeometry::default();
        let m = compose_sea_mesh(&g, 1.0);
        assert!(m.indices.is_empty());
    }

    #[test]
    fn sea_compose_all_ocean_has_tris() {
        // 3×3 grid all elev −10 → sea fill.
        let data = vec![-10.0_f32; 9];
        let g = DemVectorGrid {
            data,
            cols: 3,
            rows: 3,
            cell_x: 1.0,
            cell_y: 1.0,
            origin_x: 0.0,
            origin_y: 0.0,
            max_elev_m: -10.0,
        };
        let geo = build_sea_band_geometry(&g);
        assert!(geo.polygon_count > 0);
        let m = compose_sea_mesh(&geo, 1.0);
        assert!(!m.indices.is_empty());
        assert_eq!(m.colors.len(), m.positions.len() / 2 * 4);
    }
}
