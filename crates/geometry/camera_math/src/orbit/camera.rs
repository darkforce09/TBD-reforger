//! The doll's fixed orbit.
//!
//! **Role:** the field of view, depth planes, orbit centre, distance and eye height, and the view
//! matrix for a yaw.
//! **Position:** private to [`super`]; the projections compose with it.
//! **Signals & state:** none; constants and a pure function.
//! **Invariants:** only the yaw varies; the eye circles the centre at a fixed distance and height
//! with +y up.

use crate::matrix4::look_at;

/// Canonical fovy value.
pub(crate) const FOVY: f64 = 0.6109;

/// Canonical near value.
pub(crate) const NEAR: f64 = 0.1;

/// Canonical far value.
pub(crate) const FAR: f64 = 100.0;

/// Canonical orbit center value.
pub(crate) const ORBIT_CENTER: [f64; 3] = [0.0, 1.02, 0.0];

/// Canonical orbit dist value.
pub(crate) const ORBIT_DIST: f64 = 3.3;

/// Canonical orbit height value.
pub(crate) const ORBIT_HEIGHT: f64 = 1.45;

/// View.
pub(crate) fn view(yaw: f64) -> [f64; 16] {
    let eye = [
        ORBIT_CENTER[0] + yaw.sin() * ORBIT_DIST,
        ORBIT_HEIGHT,
        ORBIT_CENTER[2] + yaw.cos() * ORBIT_DIST,
    ];
    look_at(eye, ORBIT_CENTER, [0.0, 1.0, 0.0])
}
