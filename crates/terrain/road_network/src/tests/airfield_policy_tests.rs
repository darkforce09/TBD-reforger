//! The airfield box and the structure class gate.
//!
//! **Role:** pins the behaviour of `airfield.rs` named in each case.
//! **Position:** mounted by `airfield.rs` under `cfg(test)`.
//! **Signals & state:** none.
//! **Invariants:** every case states the exact value or event it expects.

use crate::airfield::*;
use crate::network::RoadSegment;

fn seg(id: &str, pts: &[[f64; 2]]) -> RoadSegment {
    RoadSegment {
        id: id.into(),
        road_class: "runway".to_string(),
        points: pts.to_vec(),
        width_m: 20.0,
    }
}

#[test]
fn bbox_expands_runway_union_by_margin() {
    let runways = vec![seg("r0", &[[100.0, 200.0], [300.0, 400.0]])];
    let b = compute_airfield_bbox(&runways).unwrap();
    assert_eq!(b[0], 70.0);
    assert_eq!(b[1], 170.0);
    assert_eq!(b[2], 330.0);
    assert_eq!(b[3], 430.0);
}

#[test]
fn point_in_bbox_edges() {
    let b = [0.0, 0.0, 100.0, 100.0];
    assert!(point_in_bbox(0.0, 0.0, b));
    assert!(point_in_bbox(100.0, 100.0, b));
    assert!(!point_in_bbox(-0.1, 50.0, b));
}

#[test]
fn airfield_structure_classes() {
    assert!(is_airfield_structure_class("hangar"));
    assert!(is_airfield_structure_class("tower"));
    assert!(!is_airfield_structure_class("military"));
}
