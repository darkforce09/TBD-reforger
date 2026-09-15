//! Role: projection.
//! Position: `camera/ortho` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::glmat4::identity;
use crate::camera::math::glmat4::look_at;
use crate::camera::math::glmat4::multiply;
use crate::camera::math::glmat4::ortho_no;
use crate::camera::math::glmat4::scale_in_place;
use crate::camera::math::glmat4::transform_vector;
use crate::camera::math::glmat4::translate_in_place;
use crate::camera::ortho::state::FAR;
use crate::camera::ortho::state::NEAR;
use crate::camera::ortho::state::OrthoCamera;

/// Canonical z01 value.
pub(crate) const Z01: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
];

impl OrthoCamera {
    /// View matrix uncentered.
    pub(crate) fn view_matrix_uncentered(&self) -> [f64; 16] {
        let mut u = look_at([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        scale_in_place(&mut u, [self.scale, self.scale * 1.0, self.scale]);
        u
    }
}

impl OrthoCamera {
    /// `Viewport._initMatrices`: `new Matrix4().multiplyRight(U).translate(negate(center))` with `center = projectPosition(target) = [tx, ty, 0]` (identity projectFlat, z absent).
    #[must_use]
    pub fn view_matrix(&self) -> [f64; 16] {
        let u = self.view_matrix_uncentered();
        let mut vm = multiply(&identity(), &u);
        let center = [self.target[0], self.target[1], 0.0];

        translate_in_place(&mut vm, [-center[0], -center[1], -center[2]]);
        vm
    }
}

impl OrthoCamera {
    /// `orthographic-viewport.js` `getProjectionMatrix` with `padding: null` — pure `orthoNO` over the half-extents of the (`|| 1`-coerced) viewport.
    #[must_use]
    pub fn projection_matrix(&self) -> [f64; 16] {
        let [w, h] = self.size_px();
        ortho_no(-w / 2.0, w / 2.0, -h / 2.0, h / 2.0, NEAR, FAR)
    }
}

impl OrthoCamera {
    /// `Viewport._initMatrices`: `vpm = I; vpm = vpm·P; vpm = vpm·V` — the `I·P` step is kept (it is exact but mirrored anyway for sign-of-zero fidelity).
    #[must_use]
    pub fn view_projection(&self) -> [f64; 16] {
        let p = self.projection_matrix();
        let v = self.view_matrix();
        multiply(&multiply(&identity(), &p), &v)
    }
}

impl OrthoCamera {
    /// `viewportMatrix · viewProjection` where `viewportMatrix = I.scale([w/2, -h/2, 1]) .translate([1, -1, 0])` — world → **top-left** pixel coordinates.
    #[must_use]
    pub fn pixel_projection(&self) -> [f64; 16] {
        let [w, h] = self.size_px();
        let mut viewport_m = identity();
        scale_in_place(&mut viewport_m, [w / 2.0, -h / 2.0, 1.0]);
        translate_in_place(&mut viewport_m, [1.0, -1.0, 0.0]);
        multiply(&viewport_m, &self.view_projection())
    }
}

impl OrthoCamera {
    /// `viewport.project([x, y, z])` with `topLeft: true` (deck default) → `[px, py, pz]`.
    #[must_use]
    pub fn project(&self, xyz: [f64; 3]) -> [f64; 3] {
        let z_in = xyz[2];
        let z_world = if z_in == 0.0 || z_in.is_nan() {
            0.0
        } else {
            z_in
        } * 1.0;
        let coord = transform_vector(&self.pixel_projection(), [xyz[0], xyz[1], z_world, 1.0]);
        [coord[0], coord[1], coord[2]]
    }
}

impl OrthoCamera {
    /// The render uniform: `Z01 · VP · T(anchor)`, composed in f64 and cast to f32 last.
    #[must_use]
    pub fn wgpu_clip_matrix(&self, anchor_x: f64, anchor_y: f64) -> [f32; 16] {
        let mut m = multiply(&Z01, &self.view_projection());
        translate_in_place(&mut m, [anchor_x, anchor_y, 0.0]);
        core::array::from_fn(|i| m[i] as f32)
    }
}
