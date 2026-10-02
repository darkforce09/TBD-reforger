//! The axis-aligned 3D box.
//!
//! **Role:** [`Bounds3`]: a box by its minimum and maximum corners, with the union of two boxes
//! and the box rounded to f32 precision.
//! **Position:** the bounds of a prefab's occluder descriptor in the map engine's world line of
//! sight (`spatial::los::world::descriptor`), serialised in its JSON manifest.
//! **Signals & state:** none; a plain `Copy` value.
//! **Invariants:** `min` is at or below `max` on every axis for a box built from real geometry; a
//! union keeps that; the f32 form rounds each corner to the nearest f32.

use serde::{Deserialize, Serialize};

/// An axis-aligned box in the prefab's object frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds3 {
    /// Min.
    pub min: [f64; 3],

    /// Max.
    pub max: [f64; 3],
}

impl Bounds3 {
    /// The union of two boxes.
    #[must_use]
    pub fn union(self, o: Bounds3) -> Bounds3 {
        let mut b = self;
        for a in 0..3 {
            b.min[a] = b.min[a].min(o.min[a]);
            b.max[a] = b.max[a].max(o.max[a]);
        }
        b
    }
}

impl Bounds3 {
    /// To f32 lossy.
    #[must_use]
    pub fn to_f32_lossy(self) -> Bounds3 {
        Bounds3 {
            min: self.min.map(f32_round),
            max: self.max.map(f32_round),
        }
    }
}

/// `v` rounded to the nearest f32 and widened back.
fn f32_round(v: f64) -> f64 {
    f64::from(v as f32)
}

#[cfg(test)]
#[path = "tests/axis_aligned_box.rs"]
mod tests;
