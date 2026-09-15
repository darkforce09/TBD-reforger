//! Role: mesh.
//! Position: `terrain/water` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::renderers::primitives::{
    compose::{PolyMeshGpu, mesh_from_tri},
    triangulate::triangulate_ring_buffer,
};
use crate::terrain::relief::sea_band::SeaBandGeometry;

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
