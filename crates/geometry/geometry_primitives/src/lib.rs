//! Plain f64 geometry shared by the building, line-of-sight and BVH code.
//!
//! **Role:** the 3D vector products ([`vector3`]), the 2D segment, distance and box tests over
//! plan coordinates ([`segment_geometry`]), the rigid transform of a placed object
//! ([`rigid_transform::Rigid`]) and the axis-aligned 3D box ([`axis_aligned_box::Bounds3`]).
//! **Position:** geometry tier 0, depending on `serde` only. The map engine's BVH traversal,
//! building blueprints, compound buildings, section cuts and world line of sight call it, and the
//! blueprint compiler of the developer tools builds its BVHs with it.
//! **Signals & state:** none; pure functions and plain `Copy` values.
//! **Invariants:** every value is an f64 in the caller's frame, with no map, world, terrain or GPU
//! concept; a point, vector or box is a plain array, so callers pass their own coordinates in
//! without conversion.

pub mod axis_aligned_box;
pub mod prelude;
pub mod rigid_transform;
pub mod segment_geometry;
pub mod vector3;
