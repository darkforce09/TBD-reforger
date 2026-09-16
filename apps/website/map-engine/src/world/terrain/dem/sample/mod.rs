//! Role: Module boundary for terrain/dem/sample.
//! Position: `world/terrain/dem/sample` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::world::terrain::dem::manifest::DemManifest`.
pub use crate::world::terrain::dem::manifest::DemManifest;

/// Re-export `crate::world::terrain::dem::manifest::PixelCoord`.
pub use crate::world::terrain::dem::manifest::PixelCoord;

/// Re-export `crate::world::terrain::dem::sampling::bilinear_sample`.
pub use crate::world::terrain::dem::sampling::bilinear_sample;

/// Re-export `crate::world::terrain::dem::sampling::in_coverage`.
pub use crate::world::terrain::dem::sampling::in_coverage;

/// Re-export `crate::world::terrain::dem::sampling::meters_cache`.
pub use crate::world::terrain::dem::sampling::meters_cache;

/// Re-export `crate::world::terrain::dem::sampling::sample_elevation_from_meters_cache`.
pub use crate::world::terrain::dem::sampling::sample_elevation_from_meters_cache;

/// Re-export `crate::world::terrain::dem::sampling::sample_elevation_meters`.
pub use crate::world::terrain::dem::sampling::sample_elevation_meters;

/// Re-export `crate::world::terrain::dem::sampling::uint16_to_meters`.
pub use crate::world::terrain::dem::sampling::uint16_to_meters;

/// Re-export `crate::world::terrain::dem::sampling::world_to_pixel`.
pub use crate::world::terrain::dem::sampling::world_to_pixel;

/// Re-export `crate::spatial::los::terrain::sampler::ProfileSample`.
pub use crate::spatial::los::terrain::sampler::ProfileSample;

/// Re-export `crate::spatial::los::terrain::sampler::sample_segment`.
pub use crate::spatial::los::terrain::sampler::sample_segment;

/// Re-export `crate::spatial::los::terrain::viewshed::MAX_VIEWSHED_CELLS`.
pub use crate::spatial::los::terrain::viewshed::MAX_VIEWSHED_CELLS;

/// Re-export `crate::spatial::los::terrain::viewshed::VIEWSHED_DEFAULT_RADIUS_M`.
pub use crate::spatial::los::terrain::viewshed::VIEWSHED_DEFAULT_RADIUS_M;

/// Re-export `crate::spatial::los::terrain::viewshed::Viewshed`.
pub use crate::spatial::los::terrain::viewshed::Viewshed;

/// Re-export `crate::spatial::los::terrain::viewshed::ViewshedCapRefused`.
pub use crate::spatial::los::terrain::viewshed::ViewshedCapRefused;

/// Re-export `crate::spatial::los::terrain::viewshed::ViewshedGrid`.
pub use crate::spatial::los::terrain::viewshed::ViewshedGrid;

/// Re-export `crate::spatial::los::terrain::viewshed::ViewshedParams`.
pub use crate::spatial::los::terrain::viewshed::ViewshedParams;

/// Re-export `crate::spatial::los::terrain::viewshed::Visibility`.
pub use crate::spatial::los::terrain::viewshed::Visibility;

/// Re-export `crate::spatial::los::terrain::viewshed::compute_viewshed`.
pub use crate::spatial::los::terrain::viewshed::compute_viewshed;

/// Re-export `crate::spatial::los::terrain::viewshed::viewshed_grid`.
pub use crate::spatial::los::terrain::viewshed::viewshed_grid;

/// Re-export `crate::spatial::los::terrain::scheduler::ViewshedJob`.
pub use crate::spatial::los::terrain::scheduler::ViewshedJob;
#[cfg(test)]
mod tests;
