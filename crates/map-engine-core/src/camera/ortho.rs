//! `OrthoCamera` — a bit-exact Rust mirror of the deck.gl 9.3.5 `OrthographicView` viewport
//! math as the app uses it (`useOrthographicView.ts`: `flipY: false`, scalar zoom, 2-element
//! `target`, `near`/`far`/`padding` at deck defaults).
//!
//! Composition follows `@deck.gl/core/dist/viewports/orthographic-viewport.js` +
//! `viewport.js` **operation-for-operation** (via the [`super::glmat4`] expression-tree
//! mirrors), so every output is bit-identical to the JS oracle given the same `scale` — the
//! T1/T3 gates in `tests/deckgl_ortho_parity.rs` assert ULP distance == 0. The single
//! non-mirrored operation is `scale = 2^zoom` (`Math.pow` vs Rust `exp2`, each ≤ 1 ULP from
//! exact — measured in isolation by gate T2, injectable via [`OrthoCamera::with_scale_for_test`]).
//!
//! Fixed vocabulary (plan §Ground truth): world = Arma meters, +Y = north; screen pixels
//! top-left origin, +y down; `zoom` = log2(pixels per meter); camera dimensions are **CSS
//! pixels**; matrices column-major `[f64; 16]`, vectors multiplied from the right.

use super::glmat4::{
    identity, invert, lerp2, look_at, multiply, ortho_no, scale_in_place, transform_vector,
    translate_in_place,
};
use crate::js;

/// Deck default near plane (`orthographic-viewport.js`: `near = 0.1`).
pub const NEAR: f64 = 0.1;
/// Deck default far plane (`orthographic-viewport.js`: `far = 1000`).
pub const FAR: f64 = 1000.0;
/// Zoom clamp floor — mirrors `useOrthographicView.ts` `MIN_ZOOM` (whole terrain visible).
pub const MIN_ZOOM: f64 = -6.0;
/// Zoom clamp ceiling — mirrors `useOrthographicView.ts` `MAX_ZOOM` (close inspection).
pub const MAX_ZOOM: f64 = 6.0;

/// GL→WebGPU clip-space z remap (column-major: `m[10] = 0.5`, `m[14] = 0.5`, else identity).
/// `orthoNO` targets GL NDC z ∈ [-1, 1]; WebGPU clips z outside [0, 1] — without this remap,
/// world z=0 sits at GL NDC z = −998.1/999.9 ≈ −0.9982 and the whole scene is clipped away.
/// wgpu presents WebGPU clip conventions on the WebGL backend too, so one matrix serves both.
const Z01: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0,
];

/// JS `width || 1` — deck coerces falsy (0, −0, NaN) dimensions to 1 in both
/// `OrthographicViewport` (projection args) and the `Viewport` base (`this.width`).
fn or_one(v: f64) -> f64 {
    if v == 0.0 || v.is_nan() { 1.0 } else { v }
}

/// `View.makeViewport` dimension resolution: the default `width/height: '100%'` position
/// resolves through `Math.round(position · extent / 100)` — so the app's oracle path
/// (`view.makeViewport({ width: rect.width, … })`, `TacticalMap.tsx:210`) hands the viewport
/// **rounded** dimensions (measured: 1237.33×842.67 → 1237×843; half-up at .5). JS
/// `Math.round` = half-toward-+∞ ([`js::round`]), not Rust's half-away-from-zero.
/// (Deviation note: deck's `makeViewport` returns `null` when a dimension rounds to 0; this
/// camera instead falls through to the `|| 1` coercion — sub-pixel viewports don't occur.)
fn round_dimension(v: f64) -> f64 {
    js::round(v)
}

/// `Math.min(a, b, c, d)` — NaN-propagating (Rust `f64::min` ignores NaN; JS returns NaN).
fn js_min4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    for v in [b, c, d] {
        if v.is_nan() || m.is_nan() {
            return f64::NAN;
        }
        if v < m {
            m = v;
        }
    }
    m
}

/// `Math.max(a, b, c, d)` — NaN-propagating.
fn js_max4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    for v in [b, c, d] {
        if v.is_nan() || m.is_nan() {
            return f64::NAN;
        }
        if v > m {
            m = v;
        }
    }
    m
}

/// `useOrthographicView.ts` `clamp` (line 15): `value < min ? min : value > max ? max : value`.
fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// The orthographic tactical-map camera. See the module docs for the parity contract.
#[derive(Clone, Debug)]
pub struct OrthoCamera {
    /// Viewport width in CSS pixels (raw; `|| 1` coercion happens at use sites, like deck).
    width_px: f64,
    /// Viewport height in CSS pixels (raw).
    height_px: f64,
    /// log2(pixels per meter).
    zoom: f64,
    /// Camera target (world meters). The app's view state is 2-element; deck derives
    /// `center = [tx, ty, 0]`.
    target: [f64; 2],
    /// `2^zoom`, stored so tests can inject the oracle's own value (gate T3).
    scale: f64,
    /// Optional target clamp `[minX, minY, maxX, maxY]`, mirroring `onViewStateChange`
    /// (`useOrthographicView.ts` lines 40–47). `None` ⇒ clamp-free math (the parity corpus).
    bounds: Option<[f64; 4]>,
}

impl OrthoCamera {
    /// New camera; no clamping is applied to any argument (parity corpus requirement).
    /// Dimensions go through the same `Math.round` the app's `view.makeViewport` path applies.
    #[must_use]
    pub fn new(width_px: f64, height_px: f64, target_x: f64, target_y: f64, zoom: f64) -> Self {
        Self {
            width_px: round_dimension(width_px),
            height_px: round_dimension(height_px),
            zoom,
            target: [target_x, target_y],
            scale: zoom.exp2(),
            bounds: None,
        }
    }

    /// Test-only oracle hook (plan gate T3): construct with an **injected** `scale` so the
    /// parity tests can split "pow drift" (T2) from "pipeline drift" (T3, asserted ULP == 0).
    /// Production code must never call this.
    #[doc(hidden)]
    #[must_use]
    pub fn with_scale_for_test(
        width_px: f64,
        height_px: f64,
        target_x: f64,
        target_y: f64,
        zoom: f64,
        scale: f64,
    ) -> Self {
        Self {
            width_px: round_dimension(width_px),
            height_px: round_dimension(height_px),
            zoom,
            target: [target_x, target_y],
            scale,
            bounds: None,
        }
    }

    /// Set the target clamp rect (the spike page mirrors the editor: `[0, 0, 12800, 12800]`).
    pub fn set_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.bounds = Some([min_x, min_y, max_x, max_y]);
    }

    /// Camera target x (world meters).
    #[must_use]
    pub fn target_x(&self) -> f64 {
        self.target[0]
    }

    /// Camera target y (world meters).
    #[must_use]
    pub fn target_y(&self) -> f64 {
        self.target[1]
    }

    /// Current zoom (log2 pixels per meter).
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// `2^zoom` — pixels per meter.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// Viewport size in CSS pixels after deck's `|| 1` coercion.
    #[must_use]
    pub fn size_px(&self) -> [f64; 2] {
        [or_one(self.width_px), or_one(self.height_px)]
    }

    /// Set viewport dimensions (CSS pixels; `Math.round`ed like `makeViewport`).
    pub fn resize(&mut self, width_px: f64, height_px: f64) {
        self.width_px = round_dimension(width_px);
        self.height_px = round_dimension(height_px);
    }

    /// Set the full view state, applying the view-state-layer clamps exactly as
    /// `onViewStateChange` does: zoom to [`MIN_ZOOM`], [`MAX_ZOOM`]; target to `bounds` if set.
    pub fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.zoom = clamp(zoom, MIN_ZOOM, MAX_ZOOM);
        self.scale = self.zoom.exp2();
        self.target = [target_x, target_y];
        self.clamp_target();
    }

    fn clamp_target(&mut self) {
        if let Some([min_x, min_y, max_x, max_y]) = self.bounds {
            self.target[0] = clamp(self.target[0], min_x, max_x);
            self.target[1] = clamp(self.target[1], min_y, max_y);
        }
    }

    // -- Matrices (deck's exact composition order) ---------------------------------------

    /// `OrthographicViewport`: `lookAt(eye=[0,0,1]).scale([s, s*1, s])` — the `* 1.0` mirrors
    /// `scale * (flipY ? -1 : 1)` with `flipY: false`.
    fn view_matrix_uncentered(&self) -> [f64; 16] {
        let mut u = look_at([0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        scale_in_place(&mut u, [self.scale, self.scale * 1.0, self.scale]);
        u
    }

    /// `Viewport._initMatrices`: `new Matrix4().multiplyRight(U).translate(negate(center))`
    /// with `center = projectPosition(target) = [tx, ty, 0]` (identity projectFlat, z absent).
    #[must_use]
    pub fn view_matrix(&self) -> [f64; 16] {
        let u = self.view_matrix_uncentered();
        let mut vm = multiply(&identity(), &u);
        let center = [self.target[0], self.target[1], 0.0];
        // `new Vector3(center).negate()` — including the −0.0 z.
        translate_in_place(&mut vm, [-center[0], -center[1], -center[2]]);
        vm
    }

    /// `orthographic-viewport.js` `getProjectionMatrix` with `padding: null` — pure `orthoNO`
    /// over the half-extents of the (`|| 1`-coerced) viewport.
    #[must_use]
    pub fn projection_matrix(&self) -> [f64; 16] {
        let [w, h] = self.size_px();
        ortho_no(-w / 2.0, w / 2.0, -h / 2.0, h / 2.0, NEAR, FAR)
    }

    /// `Viewport._initMatrices`: `vpm = I; vpm = vpm·P; vpm = vpm·V` — the `I·P` step is kept
    /// (it is exact but mirrored anyway for sign-of-zero fidelity).
    #[must_use]
    pub fn view_projection(&self) -> [f64; 16] {
        let p = self.projection_matrix();
        let v = self.view_matrix();
        multiply(&multiply(&identity(), &p), &v)
    }

    /// `viewportMatrix · viewProjection` where `viewportMatrix = I.scale([w/2, -h/2, 1])
    /// .translate([1, -1, 0])` — world → **top-left** pixel coordinates.
    #[must_use]
    pub fn pixel_projection(&self) -> [f64; 16] {
        let [w, h] = self.size_px();
        let mut viewport_m = identity();
        scale_in_place(&mut viewport_m, [w / 2.0, -h / 2.0, 1.0]);
        translate_in_place(&mut viewport_m, [1.0, -1.0, 0.0]);
        multiply(&viewport_m, &self.view_projection())
    }

    /// `mat4.invert(pixelProjection)` — `None` iff singular (deck logs a warning and keeps
    /// `null`; unreachable for finite non-degenerate view states).
    #[must_use]
    pub fn pixel_unprojection(&self) -> Option<[f64; 16]> {
        invert(&self.pixel_projection())
    }

    // -- Projection (mirrors `viewport.project` / `viewport.unproject`) --------------------

    /// `viewport.project([x, y, z])` with `topLeft: true` (deck default) → `[px, py, pz]`.
    #[must_use]
    pub fn project(&self, xyz: [f64; 3]) -> [f64; 3] {
        // projectPosition: Z = (xyz[2] || 0) * unitsPerMeter[2] — JS `||` coerces −0/NaN to 0.
        let z_in = xyz[2];
        let z_world = if z_in == 0.0 || z_in.is_nan() {
            0.0
        } else {
            z_in
        } * 1.0;
        let coord = transform_vector(&self.pixel_projection(), [xyz[0], xyz[1], z_world, 1.0]);
        [coord[0], coord[1], coord[2]]
    }

    /// `viewport.unproject([px, py])` (no z, `topLeft: true`) — the two-point lerp onto the
    /// world z=0 plane (`pixelsToWorld` in `@math.gl/web-mercator`). Returns `[NaN, NaN]` if
    /// the pixel matrix is singular (deck would have warned at construction).
    #[must_use]
    pub fn unproject_xy(&self, px: f64, py: f64) -> [f64; 2] {
        let Some(m_inv) = self.pixel_unprojection() else {
            return [f64::NAN, f64::NAN];
        };
        let coord0 = transform_vector(&m_inv, [px, py, 0.0, 1.0]);
        let coord1 = transform_vector(&m_inv, [px, py, 1.0, 1.0]);
        let z0 = coord0[2];
        let z1 = coord1[2];
        // `t = z0 === z1 ? 0 : ((targetZ || 0) - z0) / (z1 - z0)` with targetZ = 0.
        let t = if z0 == z1 {
            0.0
        } else {
            (0.0 - z0) / (z1 - z0)
        };
        lerp2([coord0[0], coord0[1]], [coord1[0], coord1[1]], t)
    }

    /// `viewport.getBounds()` → `[minX, minY, maxX, maxY]` — component-wise `Math.min`/`max`
    /// of the four viewport-corner unprojects at z=0 (`viewport.js` lines 188–200).
    #[must_use]
    pub fn visible_world_rect(&self) -> [f64; 4] {
        let [w, h] = self.size_px();
        let top_left = self.unproject_xy(0.0, 0.0);
        let top_right = self.unproject_xy(w, 0.0);
        let bottom_left = self.unproject_xy(0.0, h);
        let bottom_right = self.unproject_xy(w, h);
        [
            js_min4(top_left[0], top_right[0], bottom_left[0], bottom_right[0]),
            js_min4(top_left[1], top_right[1], bottom_left[1], bottom_right[1]),
            js_max4(top_left[0], top_right[0], bottom_left[0], bottom_right[0]),
            js_max4(top_left[1], top_right[1], bottom_left[1], bottom_right[1]),
        ]
    }

    // -- Interaction (the wasm event contract; invariants property-tested) -----------------

    /// Drag-pan by a screen-pixel delta — content follows the cursor. Screen +x ⇒ target
    /// west (`-= dx/scale`); screen +y (down) ⇒ target north (`+= dy/scale`, flipY:false).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.target[0] -= dx_px / self.scale;
        self.target[1] += dy_px / self.scale;
        self.clamp_target();
    }

    /// Zoom by `dz` (clamped to [[`MIN_ZOOM`], [`MAX_ZOOM`]]) keeping the world point under
    /// the cursor `(cx, cy)` (top-left CSS px) fixed on screen.
    ///
    /// Derivation (flipY:false): `px = w/2 + s·(x − tx)`, `py = h/2 − s·(y − ty)` ⇒ fixing
    /// the unproject of `(cx, cy)` at the new scale `s'` gives
    /// `tx' = wx − (cx − w/2)/s'` and `ty' = wy + (cy − h/2)/s'`.
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        let world = self.unproject_xy(cursor_x_px, cursor_y_px);
        self.zoom = clamp(self.zoom + dz, MIN_ZOOM, MAX_ZOOM);
        self.scale = self.zoom.exp2();
        let [w, h] = self.size_px();
        self.target[0] = world[0] - (cursor_x_px - w / 2.0) / self.scale;
        self.target[1] = world[1] + (cursor_y_px - h / 2.0) / self.scale;
        self.clamp_target();
    }

    // -- GPU handoff ------------------------------------------------------------------------

    /// The render uniform: `Z01 · VP · T(anchor)`, composed in f64 and cast to f32 last.
    ///
    /// `anchor` is a property of the **uploaded geometry** (vertices are stored
    /// anchor-relative in f32); the matrix carries `target − anchor` in f64. Worst-case f32
    /// error: translation magnitude ≤ 2^6 · 12800 = 819200 ⇒ relative error 2⁻²⁴ ⇒ ≤ 0.05 px
    /// at max zoom; anchor-relative scene coordinates (≤ a few hundred m) contribute < 1e-3 px.
    #[must_use]
    pub fn wgpu_clip_matrix(&self, anchor_x: f64, anchor_y: f64) -> [f32; 16] {
        let mut m = multiply(&Z01, &self.view_projection());
        translate_in_place(&mut m, [anchor_x, anchor_y, 0.0]);
        core::array::from_fn(|i| m[i] as f32)
    }
}
