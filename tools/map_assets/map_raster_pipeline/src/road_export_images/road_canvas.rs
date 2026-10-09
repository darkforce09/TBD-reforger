//! The RGBA canvas the road export images are stroked on.
//!
//! **Role:** [`RoadCanvas`] holds one image's straight (not premultiplied) RGBA pixels, blends a
//! colour into a pixel with the "over" rule ([`RoadCanvas::blend_pixel`]), stamps anti-aliased
//! discs and strokes segments with them, and fills PNG rows either as RGBA or as RGB over a solid
//! background.
//! **Position:** owned by `road_image_outputs`, which draws every layer and the junctions on these
//! canvases and streams their rows through the crate's PNG writer; the disc and stroke geometry is
//! `grid_rasterization`'s.
//! **Signals & state:** each canvas owns its pixel buffer, `width × height × 4` bytes, all zero
//! (transparent black) when created.
//! **Invariants:** pixels sit row by row from the top, four bytes each; blends happen in the
//! visit order of [`grid_rasterization::prelude::disc_coverage`]; every rounded value is a
//! weighted mean of non-negative samples, so [`f64::round`] rounds it exactly as ties toward +∞
//! would and the result stays within `0..=255`.

use grid_rasterization::prelude::{disc_coverage, segment_stamp_centres};

/// One straight-alpha RGBA image being drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RoadCanvas {
    /// The width in pixels.
    width: usize,
    /// The height in pixels.
    height: usize,
    /// The pixels, row by row from the top, `[r, g, b, a]` each.
    rgba: Vec<u8>,
}

impl RoadCanvas {
    /// A transparent `width` × `height` canvas.
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; width * height * 4],
        }
    }

    /// The `[r, g, b, a]` of pixel (`x`, `y`).
    #[cfg(test)]
    pub(super) fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let index = (y * self.width + x) * 4;
        [
            self.rgba[index],
            self.rgba[index + 1],
            self.rgba[index + 2],
            self.rgba[index + 3],
        ]
    }

    /// Blends `rgb` at `alpha` over pixel (`x`, `y`); a pixel outside the canvas is left alone.
    ///
    /// An `alpha` of 255 replaces the pixel with `rgb`, opaque. Otherwise, with `s = alpha / 255`
    /// and `d` the pixel's alpha over 255, the result's alpha is `o = s + d × (1 − s)`, stored as
    /// `round(o × 255)`, and each channel `c` over the pixel's `p` becomes
    /// `round((c × s + p × d × (1 − s)) / o)`; when `o` is 0 the pixel stays as it is.
    pub(super) fn blend_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3], alpha: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = (y * self.width + x) * 4;
        let pixel = &mut self.rgba[index..index + 4];
        if alpha == u8::MAX {
            pixel[..3].copy_from_slice(&rgb);
            pixel[3] = u8::MAX;
            return;
        }
        let source_alpha = f64::from(alpha) / 255.0;
        let destination_alpha = f64::from(pixel[3]) / 255.0;
        let destination_weight = destination_alpha * (1.0 - source_alpha);
        let output_alpha = source_alpha + destination_weight;
        if output_alpha > 0.0 {
            for (channel, source) in pixel[..3].iter_mut().zip(rgb) {
                let blended = (f64::from(source) * source_alpha
                    + f64::from(*channel) * destination_alpha * (1.0 - source_alpha))
                    / output_alpha;
                *channel = rounded_sample(blended);
            }
            pixel[3] = rounded_sample(output_alpha * 255.0);
        }
    }

    /// Stamps the anti-aliased disc of `radius` pixels at (`centre_x`, `centre_y`) in `rgb`, each
    /// covered pixel blended at the coverage alpha
    /// [`grid_rasterization::prelude::disc_coverage`] gives for `alpha`.
    pub(super) fn draw_disc(
        &mut self,
        centre_x: f64,
        centre_y: f64,
        radius: f64,
        rgb: [u8; 3],
        alpha: u8,
    ) {
        let (width, height) = (self.width, self.height);
        disc_coverage(
            centre_x,
            centre_y,
            radius,
            alpha,
            width,
            height,
            |x, y, coverage| self.blend_pixel(x, y, rgb, coverage),
        );
    }

    /// Strokes the segment from `start` to `end` (pixel coordinates) with discs of `radius` pixels
    /// at every centre [`grid_rasterization::prelude::segment_stamp_centres`] gives.
    pub(super) fn draw_segment(
        &mut self,
        start: [f64; 2],
        end: [f64; 2],
        radius: f64,
        rgb: [u8; 3],
        alpha: u8,
    ) {
        for [centre_x, centre_y] in segment_stamp_centres(start[0], start[1], end[0], end[1]) {
            self.draw_disc(centre_x, centre_y, radius, rgb, alpha);
        }
    }

    /// Copies row `row_index` into `row` as RGBA, `width × 4` bytes.
    pub(super) fn fill_rgba_row(&self, row_index: usize, row: &mut [u8]) {
        let row_length = self.width * 4;
        let start = row_index * row_length;
        row.copy_from_slice(&self.rgba[start..start + row_length]);
    }

    /// Writes row `row_index` into `row` as RGB over `background`, `width × 3` bytes: with
    /// `a` the pixel's alpha over 255, a transparent pixel is `background`, an opaque one its own
    /// colour, and any other `round(c × a + b × (1 − a))` per channel.
    pub(super) fn fill_rgb_row_over(&self, row_index: usize, row: &mut [u8], background: [u8; 3]) {
        let row_length = self.width * 4;
        let start = row_index * row_length;
        let pixels = self.rgba[start..start + row_length].chunks_exact(4);
        for (output, pixel) in row.chunks_exact_mut(3).zip(pixels) {
            let alpha = f64::from(pixel[3]) / 255.0;
            if alpha <= 0.0 {
                output.copy_from_slice(&background);
            } else if alpha >= 1.0 {
                output.copy_from_slice(&pixel[..3]);
            } else {
                for ((sample, &colour), backdrop) in
                    output.iter_mut().zip(&pixel[..3]).zip(background)
                {
                    *sample = rounded_sample(
                        f64::from(colour) * alpha + f64::from(backdrop) * (1.0 - alpha),
                    );
                }
            }
        }
    }
}

/// The 8-bit sample of `value`, a non-negative weighted mean of 8-bit samples: rounded with ties
/// toward +∞ (which [`f64::round`] equals on non-negative values), the cast saturating at 255.
fn rounded_sample(value: f64) -> u8 {
    value.round() as u8
}

#[cfg(test)]
#[path = "../tests/road_export_images_canvas_tests.rs"]
mod tests;
