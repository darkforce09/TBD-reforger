//! Role: projection.
//! Position: `camera/orbit` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::glmat4::multiply;
use crate::camera::math::glmat4::perspective_no;
use crate::camera::orbit::camera::FAR;
use crate::camera::orbit::camera::FOVY;
use crate::camera::orbit::camera::NEAR;
use crate::camera::orbit::camera::view;

/// Canonical z01 value.
pub(crate) const Z01: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
];

/// `P · V` in GL clip conventions (pick path — unproject uses NDC z ∈ [-1, 1]).
#[must_use]
pub fn view_proj_gl(yaw: f64, w_px: f64, h_px: f64) -> [f64; 16] {
    let aspect = if h_px <= 0.0 { 1.0 } else { w_px / h_px };
    multiply(&perspective_no(FOVY, aspect, NEAR, FAR), &view(yaw))
}

/// The render uniform: `Z01 · P · V`, f64-composed, cast to f32 last (per-instance model matrices multiply in the shader).
#[must_use]
pub fn view_proj_wgpu(yaw: f64, w_px: f64, h_px: f64) -> [f32; 16] {
    let m = multiply(&Z01, &view_proj_gl(yaw, w_px, h_px));
    core::array::from_fn(|i| m[i] as f32)
}
