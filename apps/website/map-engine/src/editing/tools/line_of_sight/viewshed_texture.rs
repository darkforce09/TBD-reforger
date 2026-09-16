//! Role: pack a computed viewshed into the bytes a texture upload wants.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: rows are padded to the 256-byte copy alignment the GPU copy requires; the world rect travels with the bytes so the caller needs no raster arithmetic.

use crate::spatial::los::terrain::viewshed::Viewshed;

use super::wash_palette::encode_viewshed_rgba;

/// The world rect + RGBA bytes for a viewshed texture upload. Returned by
/// [`viewshed_texture_payload`] so the host's wiring is a mechanical
/// `viewshed_upload(rect…, w, h, &rgba, stride)` with no DEM or encode logic of its own.
/// `stride_bytes` is the 256-aligned `bytes_per_row` the raster was packed to — see
/// [`pack_rgba_256`].
#[derive(Clone, Debug, PartialEq)]
pub struct ViewshedTexture {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub tex_w: u32,
    pub tex_h: u32,
    /// Row-padded RGBA8 (`stride_bytes * tex_h` bytes).
    pub rgba: Vec<u8>,
    /// 256-aligned bytes-per-row (the engine's `write_texture` copy requirement).
    pub stride_bytes: u32,
}

/// Row-pad a tightly-packed `cols*rows*4` RGBA buffer to a 256-aligned `bytes_per_row` — the
/// `write_texture` copy requirement a viewshed upload enforces. Returns `(padded_bytes, stride)`.
/// A width already 256-aligned (`cols*4 % 256 == 0`)
/// is copied through unchanged; otherwise each row is right-padded with zero bytes (fully transparent
/// texels — they fall outside the drawn cells anyway since the quad UVs span only `cols`).
#[must_use]
pub fn pack_rgba_256(tight: &[u8], cols: usize, rows: usize) -> (Vec<u8>, u32) {
    let row_bytes = cols * 4;
    let stride = row_bytes.div_ceil(256) * 256;
    if stride == row_bytes {
        return (tight.to_vec(), stride as u32);
    }
    let mut out = vec![0u8; stride * rows];
    for r in 0..rows {
        let src = &tight[r * row_bytes..(r + 1) * row_bytes];
        out[r * stride..r * stride + row_bytes].copy_from_slice(src);
    }
    (out, stride as u32)
}

/// Build the upload-ready viewshed texture payload from a computed raster: encode via the palette
/// ([`encode_viewshed_rgba`]) then row-pad to 256 ([`pack_rgba_256`]). The one call a host makes
/// after a compute to get bytes it can hand straight to a texture upload.
#[must_use]
pub fn viewshed_texture_payload(vs: &Viewshed) -> ViewshedTexture {
    let tight = encode_viewshed_rgba(vs);
    let (rgba, stride_bytes) = pack_rgba_256(&tight, vs.cols, vs.rows);
    ViewshedTexture {
        min_x: vs.min_x,
        min_y: vs.min_y,
        max_x: vs.max_x,
        max_y: vs.max_y,
        tex_w: vs.cols as u32,
        tex_h: vs.rows as u32,
        rgba,
        stride_bytes,
    }
}

/// Place a viewshed observer at world `(x, y)` and return the texture payload to upload.
///
/// It SUBMITS the raster to the [`viewshed_scheduler`](super::super::viewshed_scheduler) — which
/// caps the request, cancels whatever was computing, runs one budgeted batch and publishes the
/// raster into the registered viewshed state (so a pan re-projects the same rect without a
/// recompute), then finishes the disc across later frames — and returns the [`ViewshedTexture`] for
/// the raster so far. `None` when no sampler is registered or when a cap refused the request; the
/// host then draws nothing, and `viewshed_scheduler::last_refusal` names the cap.
#[must_use]
pub fn place_viewshed(x: f64, y: f64) -> Option<ViewshedTexture> {
    let vs = super::super::viewshed_scheduler::submit_terrain(x, y)?;
    Some(viewshed_texture_payload(&vs))
}

#[cfg(test)]
#[path = "tests/viewshed_texture.rs"]
mod tests;
