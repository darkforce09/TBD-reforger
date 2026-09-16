//! Role: bounds.
//! Position: `spatial/world_los/descriptor` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::Deserialize;
use super::Serialize;

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

/// F32 round.
pub(super) fn f32_round(v: f64) -> f64 {
    f64::from(v as f32)
}
