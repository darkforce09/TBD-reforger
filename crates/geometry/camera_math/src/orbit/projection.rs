//! The doll's view-projections.
//!
//! **Role:** [`view_proj_gl`] in GL clip space for picking and [`view_proj_wgpu`] for the render
//! uniform.
//! **Position:** called by the map engine's doll renderer, picking and scene model and its doll
//! readback check.
//! **Signals & state:** none; pure functions of the yaw and the viewport size.
//! **Invariants:** the render form is the picking form with the `Z01` depth remap in front,
//! composed in f64 and cast to f32 last; a non-positive height takes an aspect of 1.

use crate::matrix4::multiply;
use crate::matrix4::perspective_no;
use crate::orbit::camera::FAR;
use crate::orbit::camera::FOVY;
use crate::orbit::camera::NEAR;
use crate::orbit::camera::view;

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
