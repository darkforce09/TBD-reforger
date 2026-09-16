//! Role: damage tests.
//! Position: `core/pipeline/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::pipeline::damage::*;

#[test]
fn class_r_clean_second_frame_skips() {
    let mut d = RenderDamage::new();
    assert!(d.begin_frame().submit);
    d.after_submit();
    assert!(!d.dirty);
    assert!(!d.begin_frame().submit);
}

#[test]
fn class_r_pan_marks_dirty() {
    let mut d = RenderDamage::new();
    d.after_submit();
    assert!(!d.begin_frame().submit);
    d.mark();
    assert!(d.begin_frame().submit);
}

#[test]
fn class_r_continuous_always_submits() {
    let mut d = RenderDamage::new();
    d.set_continuous(true);
    d.after_submit();
    assert!(d.begin_frame().submit);
    d.after_submit();
    assert!(d.begin_frame().submit);
}

#[test]
fn class_r_skip_leaves_dirty_false() {
    let d = RenderDamage {
        dirty: false,
        continuous: false,
    };
    assert!(!d.begin_frame().submit);
    assert!(!d.dirty);
}
