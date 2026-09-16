//! Role: Module boundary for terrain/dem/sample.
//! Position: `terrain/dem/sample` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::terrain::dem::manifest::DemManifest`.
pub use crate::terrain::dem::manifest::DemManifest;

/// Re-export `crate::terrain::dem::manifest::PixelCoord`.
pub use crate::terrain::dem::manifest::PixelCoord;

/// Re-export `crate::terrain::dem::sampling::bilinear_sample`.
pub use crate::terrain::dem::sampling::bilinear_sample;

/// Re-export `crate::terrain::dem::sampling::in_coverage`.
pub use crate::terrain::dem::sampling::in_coverage;

/// Re-export `crate::terrain::dem::sampling::meters_cache`.
pub use crate::terrain::dem::sampling::meters_cache;

/// Re-export `crate::terrain::dem::sampling::sample_elevation_from_meters_cache`.
pub use crate::terrain::dem::sampling::sample_elevation_from_meters_cache;

/// Re-export `crate::terrain::dem::sampling::sample_elevation_meters`.
pub use crate::terrain::dem::sampling::sample_elevation_meters;

/// Re-export `crate::terrain::dem::sampling::uint16_to_meters`.
pub use crate::terrain::dem::sampling::uint16_to_meters;

/// Re-export `crate::terrain::dem::sampling::world_to_pixel`.
pub use crate::terrain::dem::sampling::world_to_pixel;

/// Re-export `crate::spatial::terrain_los::sampler::ProfileSample`.
pub use crate::spatial::terrain_los::sampler::ProfileSample;

/// Re-export `crate::spatial::terrain_los::sampler::sample_segment`.
pub use crate::spatial::terrain_los::sampler::sample_segment;

/// Re-export `crate::spatial::terrain_los::viewshed::MAX_VIEWSHED_CELLS`.
pub use crate::spatial::terrain_los::viewshed::MAX_VIEWSHED_CELLS;

/// Re-export `crate::spatial::terrain_los::viewshed::VIEWSHED_DEFAULT_RADIUS_M`.
pub use crate::spatial::terrain_los::viewshed::VIEWSHED_DEFAULT_RADIUS_M;

/// Re-export `crate::spatial::terrain_los::viewshed::Viewshed`.
pub use crate::spatial::terrain_los::viewshed::Viewshed;

/// Re-export `crate::spatial::terrain_los::viewshed::ViewshedCapRefused`.
pub use crate::spatial::terrain_los::viewshed::ViewshedCapRefused;

/// Re-export `crate::spatial::terrain_los::viewshed::ViewshedGrid`.
pub use crate::spatial::terrain_los::viewshed::ViewshedGrid;

/// Re-export `crate::spatial::terrain_los::viewshed::ViewshedParams`.
pub use crate::spatial::terrain_los::viewshed::ViewshedParams;

/// Re-export `crate::spatial::terrain_los::viewshed::Visibility`.
pub use crate::spatial::terrain_los::viewshed::Visibility;

/// Re-export `crate::spatial::terrain_los::viewshed::compute_viewshed`.
pub use crate::spatial::terrain_los::viewshed::compute_viewshed;

/// Re-export `crate::spatial::terrain_los::viewshed::viewshed_grid`.
pub use crate::spatial::terrain_los::viewshed::viewshed_grid;

/// Re-export `crate::spatial::terrain_los::scheduler::ViewshedJob`.
pub use crate::spatial::terrain_los::scheduler::ViewshedJob;
#[cfg(test)]
mod tests;
