//! The sea fill mesh.
//!
//! **Role:** [`compose_sea_mesh`] triangulates the sea band's rings, with their per-vertex
//! colours, into a fill mesh at the layer's opacity.
//! **Position:** reads `terrain_relief`'s [`SeaBandGeometry`] and shapes the mesh with
//! `render_primitives`; the map engine's relief host uploads it to the sea lane.
//! **Signals & state:** none; a pure function.
//! **Invariants:** an empty band, or an opacity at or below zero, yields the empty mesh.

use render_primitives::draw::compose::{PolyMeshGpu, mesh_from_tri};
use render_primitives::draw::triangulate::triangulate_ring_buffer;
use terrain_relief::sea_band::SeaBandGeometry;

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
