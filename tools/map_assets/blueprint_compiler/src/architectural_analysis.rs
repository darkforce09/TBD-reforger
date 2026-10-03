//! The geometry stages of the blueprint compiler.
//!
//! **Role:** from a building's voxel dump, finds the floor slabs, the walls of each floor band,
//! the walkable floor plates and their outlines and the roof heightfield; for the collision
//! meshes, rebuilds convex colliders and classifies each triangle as opaque, glass or foliage;
//! and holds the march skeleton every voxel dump generator shares.
//! **Position:** fed by [`crate::voxel_processing`] (the dump model and the tunables) and
//! [`crate::mesh_decoding`] (collider hulls, material stems); read by
//! [`crate::blueprint_from_voxels`], [`crate::archive_emission`] and [`crate::bvh`].
//! **Signals & state:** none; pure functions over the dump and the tunables.
//! **Invariants:** every stage works in the dump's normalized frame (origin at the dump's
//! minimum corner); a stage never reads a file.

pub(crate) mod contour_tracing;
pub(crate) mod convex_hulls;
pub(crate) mod floor_plates;
pub(crate) mod polygon_rings;
pub(crate) mod roof_profiles;
pub(crate) mod surface_classification;
pub(crate) mod vertical_slabs;
pub(crate) mod wall_extraction;
