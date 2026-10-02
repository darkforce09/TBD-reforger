//! Products and differences of 3D vectors.
//!
//! **Role:** the vector arithmetic of the ray–triangle test, the BVH traversal and the section
//! cutter: difference, cross product and dot product of `[f64; 3]` values.
//! **Position:** called by the map engine's `spatial::bvh` (node test and traversal),
//! `world::architecture::section` (the cutter) and the blueprint compiler's BVH construction in
//! the developer tools.
//! **Signals & state:** none; pure functions.
//! **Invariants:** each component is computed in the operand order written here, so the bits of a
//! result do not depend on the caller; a right-handed `cross` (`x × y = z`).

/// The difference `a − b`, component by component.
#[must_use]
pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The right-handed cross product `a × b`.
#[must_use]
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The dot product `a · b`.
#[must_use]
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
