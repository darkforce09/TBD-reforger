//! Role: the crate's one home for colour normalisation: RGBA8 channels to linear 0..1 floats,
//! plain ([`norm`]) and with a layer alpha folded into the alpha channel ([`u8_rgba_to_f32`]).
//! Position: crate root of `render_primitives`; `draw::grid` and `draw::compose` build their
//! vertex colours with it, and callers that write the same vertex layouts use it too.
//! Signals & state: none; pure functions.
//! Invariants: each channel maps `0..=255` to `0.0..=1.0` by division by 255 with no transfer
//! function (the render target is not sRGB); only the alpha channel is ever scaled by a layer
//! alpha.

/// RGBA8 → linear 0..1 floats.
#[must_use]
pub fn norm(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

/// RGBA8 → linear 0..1, with the layer's alpha folded into the alpha channel.
#[must_use]
pub fn u8_rgba_to_f32(c: [u8; 4], layer_alpha: f32) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        (f32::from(c[3]) / 255.0) * layer_alpha,
    ]
}
