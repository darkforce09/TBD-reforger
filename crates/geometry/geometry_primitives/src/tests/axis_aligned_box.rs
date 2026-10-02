//! **Role:** unit tests for [`crate::axis_aligned_box::Bounds3`]: the union of two boxes.
//! **Position:** `src/tests` of `geometry_primitives`, declared by `axis_aligned_box.rs`.
//! **Signals & state:** none; literal boxes.
//! **Invariants:** every expected corner is written out by hand.

use crate::axis_aligned_box::Bounds3;

#[test]
fn bounds_union_is_componentwise() {
    let a = Bounds3 {
        min: [0.0, 0.0, 0.0],
        max: [1.0, 1.0, 1.0],
    };
    let b = Bounds3 {
        min: [-1.0, 0.5, 0.0],
        max: [0.5, 2.0, 3.0],
    };
    assert_eq!(
        a.union(b),
        Bounds3 {
            min: [-1.0, 0.0, 0.0],
            max: [1.0, 2.0, 3.0]
        }
    );
}
