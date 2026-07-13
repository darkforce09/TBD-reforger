//! Verbatim f64 mirrors of the exact gl-matrix routines deck.gl's viewport math executes
//! (the copy vendored in `@math.gl/core/dist/gl-matrix/mat4.js` — line references below are
//! into that file at the pinned `@math.gl/core` used by `@deck.gl/core@9.3.5`).
//!
//! **Contract:** every function preserves the JS **expression tree** operation-for-operation
//! (same multiplies, same adds, same association). IEEE-754 double ops are deterministic, so
//! identical expression trees ⇒ identical bit patterns — this is what makes the T1/T3 parity
//! gates (`tests/deckgl_ortho_parity.rs`) legitimately assert ULP distance == 0 against the
//! deck.gl oracle. Do NOT "simplify" arithmetic here (e.g. `-2.0 * lr` → `2.0 / w`): any
//! algebraic rewrite can change the low bit and break bit-exactness.
//!
//! Matrices are column-major `[f64; 16]`, vectors multiply from the right — gl-matrix layout.

/// `glMatrix.EPSILON` (gl-matrix `common.js`).
const EPSILON: f64 = 0.000_001;

/// Column-major identity, mirroring deck's `createMat4()` (`@deck.gl/core` `utils/math-utils.js`
/// returns a plain JS array — f64, not Float32Array).
#[must_use]
pub fn identity() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// `mat4.multiply(out, a, b)` — mat4.js:401. Returns `a · b` (b applied first to vectors).
/// gl-matrix caches all of `a` and each column of `b` before writing, so its in-place uses
/// (`out === a`) are equivalent to this pure form.
#[must_use]
#[allow(clippy::similar_names)]
pub fn multiply(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    let a00 = a[0];
    let a01 = a[1];
    let a02 = a[2];
    let a03 = a[3];
    let a10 = a[4];
    let a11 = a[5];
    let a12 = a[6];
    let a13 = a[7];
    let a20 = a[8];
    let a21 = a[9];
    let a22 = a[10];
    let a23 = a[11];
    let a30 = a[12];
    let a31 = a[13];
    let a32 = a[14];
    let a33 = a[15];

    let mut out = [0.0; 16];
    let (mut b0, mut b1, mut b2, mut b3) = (b[0], b[1], b[2], b[3]);
    out[0] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
    out[1] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
    out[2] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
    out[3] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;
    b0 = b[4];
    b1 = b[5];
    b2 = b[6];
    b3 = b[7];
    out[4] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
    out[5] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
    out[6] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
    out[7] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;
    b0 = b[8];
    b1 = b[9];
    b2 = b[10];
    b3 = b[11];
    out[8] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
    out[9] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
    out[10] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
    out[11] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;
    b0 = b[12];
    b1 = b[13];
    b2 = b[14];
    b3 = b[15];
    out[12] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
    out[13] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
    out[14] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
    out[15] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;
    out
}

/// `mat4.translate(out, a, v)` with `out === a` — mat4.js:461 in-place branch (the only branch
/// deck's `Matrix4.translate` ever takes: it calls `mat4_translate(this, this, vector)`).
// The `x = a + x` form is kept verbatim from the JS (module contract); clippy's `+=` rewrite
// would be bit-identical (IEEE addition commutes) but breaks the line-for-line citation.
#[expect(clippy::assign_op_pattern)]
pub fn translate_in_place(m: &mut [f64; 16], v: [f64; 3]) {
    let (x, y, z) = (v[0], v[1], v[2]);
    m[12] = m[0] * x + m[4] * y + m[8] * z + m[12];
    m[13] = m[1] * x + m[5] * y + m[9] * z + m[13];
    m[14] = m[2] * x + m[6] * y + m[10] * z + m[14];
    m[15] = m[3] * x + m[7] * y + m[11] * z + m[15];
}

/// `mat4.scale(out, a, v)` with `out === a` — mat4.js:523 (deck's `Matrix4.scale` calls
/// `mat4_scale(this, this, v)`).
pub fn scale_in_place(m: &mut [f64; 16], v: [f64; 3]) {
    let (x, y, z) = (v[0], v[1], v[2]);
    m[0] *= x;
    m[1] *= x;
    m[2] *= x;
    m[3] *= x;
    m[4] *= y;
    m[5] *= y;
    m[6] *= y;
    m[7] *= y;
    m[8] *= z;
    m[9] *= z;
    m[10] *= z;
    m[11] *= z;
}

/// `mat4.orthoNO(out, left, right, bottom, top, near, far)` — mat4.js:1555 (`mat4.ortho` alias;
/// GL clip convention, NDC z ∈ [-1, 1]). Deck's `Matrix4.ortho` delegates here.
#[must_use]
pub fn ortho_no(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> [f64; 16] {
    let lr = 1.0 / (left - right);
    let bt = 1.0 / (bottom - top);
    let nf = 1.0 / (near - far);
    let mut out = [0.0; 16];
    out[0] = -2.0 * lr;
    out[5] = -2.0 * bt;
    out[10] = 2.0 * nf;
    out[12] = (left + right) * lr;
    out[13] = (top + bottom) * bt;
    out[14] = (far + near) * nf;
    out[15] = 1.0;
    out
}

/// `mat4.perspectiveNO(out, fovy, aspect, near, far)` — mat4.js:1411 (`mat4.perspective`
/// alias; GL clip convention, NDC z ∈ [-1, 1]), including the finite-far / infinite-far
/// branch. First perspective consumer is the T-154 doll engine (the map stays orthographic).
#[must_use]
pub fn perspective_no(fovy: f64, aspect: f64, near: f64, far: f64) -> [f64; 16] {
    let f = 1.0 / (fovy / 2.0).tan();
    let mut out = [0.0; 16];
    out[0] = f / aspect;
    out[5] = f;
    out[11] = -1.0;
    // gl-matrix: `if (far != null && far !== Infinity)`.
    if far.is_finite() {
        let nf = 1.0 / (near - far);
        out[10] = (far + near) * nf;
        out[14] = 2.0 * far * near * nf;
    } else {
        out[10] = -1.0;
        out[14] = -2.0 * near;
    }
    out
}

/// `mat4.lookAt(out, eye, center, up)` — mat4.js:1628, including the `glMatrix.EPSILON`
/// identity short-circuit and the zero-length cross-product guards.
#[must_use]
pub fn look_at(eye: [f64; 3], center: [f64; 3], up: [f64; 3]) -> [f64; 16] {
    let (eyex, eyey, eyez) = (eye[0], eye[1], eye[2]);
    let (upx, upy, upz) = (up[0], up[1], up[2]);
    let (centerx, centery, centerz) = (center[0], center[1], center[2]);

    if (eyex - centerx).abs() < EPSILON
        && (eyey - centery).abs() < EPSILON
        && (eyez - centerz).abs() < EPSILON
    {
        return identity();
    }

    let mut z0 = eyex - centerx;
    let mut z1 = eyey - centery;
    let mut z2 = eyez - centerz;
    let mut len = 1.0 / (z0 * z0 + z1 * z1 + z2 * z2).sqrt();
    z0 *= len;
    z1 *= len;
    z2 *= len;

    let mut x0 = upy * z2 - upz * z1;
    let mut x1 = upz * z0 - upx * z2;
    let mut x2 = upx * z1 - upy * z0;
    len = (x0 * x0 + x1 * x1 + x2 * x2).sqrt();
    if len == 0.0 {
        x0 = 0.0;
        x1 = 0.0;
        x2 = 0.0;
    } else {
        len = 1.0 / len;
        x0 *= len;
        x1 *= len;
        x2 *= len;
    }

    let mut y0 = z1 * x2 - z2 * x1;
    let mut y1 = z2 * x0 - z0 * x2;
    let mut y2 = z0 * x1 - z1 * x0;
    len = (y0 * y0 + y1 * y1 + y2 * y2).sqrt();
    if len == 0.0 {
        y0 = 0.0;
        y1 = 0.0;
        y2 = 0.0;
    } else {
        len = 1.0 / len;
        y0 *= len;
        y1 *= len;
        y2 *= len;
    }

    [
        x0,
        y0,
        z0,
        0.0,
        x1,
        y1,
        z1,
        0.0,
        x2,
        y2,
        z2,
        0.0,
        -(x0 * eyex + x1 * eyey + x2 * eyez),
        -(y0 * eyex + y1 * eyey + y2 * eyez),
        -(z0 * eyex + z1 * eyey + z2 * eyez),
        1.0,
    ]
}

/// `mat4.invert(out, a)` — mat4.js:250 (cofactor expansion, exact expression tree). Returns
/// `None` when the determinant is zero (gl-matrix returns `null`).
#[must_use]
#[allow(clippy::similar_names)]
pub fn invert(a: &[f64; 16]) -> Option<[f64; 16]> {
    let a00 = a[0];
    let a01 = a[1];
    let a02 = a[2];
    let a03 = a[3];
    let a10 = a[4];
    let a11 = a[5];
    let a12 = a[6];
    let a13 = a[7];
    let a20 = a[8];
    let a21 = a[9];
    let a22 = a[10];
    let a23 = a[11];
    let a30 = a[12];
    let a31 = a[13];
    let a32 = a[14];
    let a33 = a[15];

    let b00 = a00 * a11 - a01 * a10;
    let b01 = a00 * a12 - a02 * a10;
    let b02 = a00 * a13 - a03 * a10;
    let b03 = a01 * a12 - a02 * a11;
    let b04 = a01 * a13 - a03 * a11;
    let b05 = a02 * a13 - a03 * a12;
    let b06 = a20 * a31 - a21 * a30;
    let b07 = a20 * a32 - a22 * a30;
    let b08 = a20 * a33 - a23 * a30;
    let b09 = a21 * a32 - a22 * a31;
    let b10 = a21 * a33 - a23 * a31;
    let b11 = a22 * a33 - a23 * a32;

    let mut det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
    // gl-matrix: `if (!det) return null;` — JS falsy covers ±0 and NaN.
    if det == 0.0 || det.is_nan() {
        return None;
    }
    det = 1.0 / det;

    Some([
        (a11 * b11 - a12 * b10 + a13 * b09) * det,
        (a02 * b10 - a01 * b11 - a03 * b09) * det,
        (a31 * b05 - a32 * b04 + a33 * b03) * det,
        (a22 * b04 - a21 * b05 - a23 * b03) * det,
        (a12 * b08 - a10 * b11 - a13 * b07) * det,
        (a00 * b11 - a02 * b08 + a03 * b07) * det,
        (a32 * b02 - a30 * b05 - a33 * b01) * det,
        (a20 * b05 - a22 * b02 + a23 * b01) * det,
        (a10 * b10 - a11 * b08 + a13 * b06) * det,
        (a01 * b08 - a00 * b10 - a03 * b06) * det,
        (a30 * b04 - a31 * b02 + a33 * b00) * det,
        (a21 * b02 - a20 * b04 - a23 * b00) * det,
        (a11 * b07 - a10 * b09 - a12 * b06) * det,
        (a00 * b09 - a01 * b07 + a02 * b06) * det,
        (a31 * b01 - a30 * b03 - a32 * b00) * det,
        (a20 * b03 - a21 * b01 + a22 * b00) * det,
    ])
}

/// `transformVector(matrix, vector)` — `@math.gl/web-mercator/dist/math-utils.js`:
/// `vec4.transformMat4` (vec4.js) followed by `vec4.scale(result, result, 1 / result[3])`.
/// **Expression-tree detail:** the w-divide is `component * (1/w)` — a single reciprocal then
/// multiplies — NOT `component / w`; the two differ bitwise.
#[must_use]
pub fn transform_vector(m: &[f64; 16], v: [f64; 4]) -> [f64; 4] {
    let (x, y, z, w) = (v[0], v[1], v[2], v[3]);
    let mut out = [
        m[0] * x + m[4] * y + m[8] * z + m[12] * w,
        m[1] * x + m[5] * y + m[9] * z + m[13] * w,
        m[2] * x + m[6] * y + m[10] * z + m[14] * w,
        m[3] * x + m[7] * y + m[11] * z + m[15] * w,
    ];
    let inv_w = 1.0 / out[3];
    out[0] *= inv_w;
    out[1] *= inv_w;
    out[2] *= inv_w;
    out[3] *= inv_w;
    out
}

/// `vec2.lerp(out, a, b, t)` — gl-matrix vec2.js: `out[i] = a[i] + t * (b[i] - a[i])`.
#[must_use]
pub fn lerp2(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
}
