//! Drawing the road layers onto their canvases and writing the export images.
//!
//! **Role:** [`draw_and_write_road_images`] strokes every layer in
//! [`ROAD_EXPORT_DRAW_ORDER`] onto the master canvas and onto a canvas of its own, writes each
//! layer image as it finishes, marks the junctions on the master, then writes the transparent and
//! the dark master images.
//! **Position:** called by the lane root `road_export_images::run` with the layers and junctions
//! of `road_layer_loading`; draws on `road_canvas` and writes through
//! `image_operations::png_writing`.
//! **Signals & state:** none held; the master canvas lives for one call, each layer canvas only
//! while its layer is drawn.
//! **Invariants:** world `(x, z)` maps to pixel `((x / world) × (size − 1), ((world − z) / world) ×
//! (size − 1))`, north up; a stroke's radius is `max(min width / 2, (width m / 2) / (world /
//! size))`, NaN when either side is NaN; every pair of consecutive points is one stroke, drawn on
//! the master first and then on the layer canvas; a layer image is written for every layer that
//! lists a segment, drawable or not; junction marks go on the master images only.

use std::path::Path;

use road_network::prelude::{
    ROAD_EXPORT_DARK_BACKGROUND_RGB, ROAD_EXPORT_DRAW_ORDER, ROAD_JUNCTION_ALPHA,
    ROAD_JUNCTION_MIN_DEGREE, ROAD_JUNCTION_RADIUS_PX, ROAD_JUNCTION_RGB, road_export_image_style,
};

use super::road_canvas::RoadCanvas;
use super::road_layer_loading::{RoadJunction, RoadLayer};
use crate::error::{Result, ResultExt};
use crate::image_operations::png_writing::{PngPixelLayout, write_png_rows};

/// Where and how large the images of one run are written, and the world they cover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RoadImageSet<'a> {
    /// The output folder, which exists.
    pub(super) out_dir: &'a Path,
    /// The width and height of every image, in pixels; at least 1.
    pub(super) size_px: u32,
    /// The side of the square world, in metres.
    pub(super) world_size_m: f64,
    /// The prefix of the two master images.
    pub(super) terrain: &'a str,
}

impl RoadImageSet<'_> {
    /// The pixel position of the world plan position (`x`, `z`), north up.
    fn world_to_screen(&self, [world_x, world_z]: [f64; 2]) -> [f64; 2] {
        let last_pixel = f64::from(self.size_px.saturating_sub(1));
        let world = self.world_size_m;
        [
            (world_x / world) * last_pixel,
            ((world - world_z) / world) * last_pixel,
        ]
    }

    /// The world metres one pixel spans.
    fn metres_per_pixel(&self) -> f64 {
        self.world_size_m / f64::from(self.size_px)
    }
}

/// Draws `layers` in [`ROAD_EXPORT_DRAW_ORDER`] onto the master canvas and writes
/// `layer-<file stem>.png` for each layer that lists a segment; then marks every junction of
/// `junctions` whose degree is at least [`ROAD_JUNCTION_MIN_DEGREE`] on the master and writes
/// `<terrain>-roads-transparent.png` (RGBA) and `<terrain>-roads-dark.png` (RGB over
/// [`ROAD_EXPORT_DARK_BACKGROUND_RGB`]). A failed write is an [`crate::error::Error`].
pub(super) fn draw_and_write_road_images(
    image_set: &RoadImageSet<'_>,
    layers: &[RoadLayer],
    junctions: &[RoadJunction],
) -> Result<()> {
    let size = image_set.size_px as usize;
    println!("Scale: {:.2} meters/pixel", image_set.metres_per_pixel());
    let mut master = RoadCanvas::new(size, size);
    for road_class in ROAD_EXPORT_DRAW_ORDER {
        let Some(layer) = layers
            .iter()
            .find(|layer| layer.road_class == road_class && layer.listed_segment_count > 0)
        else {
            continue;
        };
        let mut layer_canvas = RoadCanvas::new(size, size);
        draw_layer(image_set, layer, &mut master, &mut layer_canvas)?;
        let layer_path = image_set
            .out_dir
            .join(format!("layer-{}.png", layer.file_stem));
        write_rgba_image(&layer_path, image_set.size_px, &layer_canvas)?;
        println!("Saved isolated layer: {}", layer_path.display());
    }

    if !junctions.is_empty() {
        println!("Rendering {} road junctions...", junctions.len());
        draw_junctions(image_set, junctions, &mut master);
    }

    let transparent_path = image_set
        .out_dir
        .join(format!("{}-roads-transparent.png", image_set.terrain));
    write_rgba_image(&transparent_path, image_set.size_px, &master)?;
    println!(
        "\nSaved Transparent Road Map: {} ({})",
        transparent_path.display(),
        file_size_text(&transparent_path)
    );
    let dark_path = image_set
        .out_dir
        .join(format!("{}-roads-dark.png", image_set.terrain));
    write_png_rows(
        &dark_path,
        image_set.size_px,
        image_set.size_px,
        PngPixelLayout::Rgb8,
        |row_index, row| {
            master.fill_rgb_row_over(row_index, row, ROAD_EXPORT_DARK_BACKGROUND_RGB);
        },
    )?;
    println!(
        "Saved Dark Mode Road Map   : {} ({})",
        dark_path.display(),
        file_size_text(&dark_path)
    );
    Ok(())
}

/// Strokes every segment of `layer` in its class style, each pair of consecutive points onto
/// `master` and then onto `layer_canvas`.
fn draw_layer(
    image_set: &RoadImageSet<'_>,
    layer: &RoadLayer,
    master: &mut RoadCanvas,
    layer_canvas: &mut RoadCanvas,
) -> Result<()> {
    let style = road_export_image_style(layer.road_class).with_context(|| {
        format!(
            "road class `{}` has no export image style",
            layer.road_class
        )
    })?;
    let [red, green, blue, alpha] = style.rgba;
    let rgb = [red, green, blue];
    let metres_per_pixel = image_set.metres_per_pixel();
    for segment in &layer.segments {
        let width_m = segment.width_m.unwrap_or(style.width_m);
        let radius = maximum_or_nan(style.min_width_px * 0.5, (width_m * 0.5) / metres_per_pixel);
        for pair in segment.world_points.windows(2) {
            let start = image_set.world_to_screen(pair[0]);
            let end = image_set.world_to_screen(pair[1]);
            master.draw_segment(start, end, radius, rgb, alpha);
            layer_canvas.draw_segment(start, end, radius, rgb, alpha);
        }
    }
    Ok(())
}

/// Marks each junction of at least [`ROAD_JUNCTION_MIN_DEGREE`] segments with a disc of
/// [`ROAD_JUNCTION_RADIUS_PX`] in [`ROAD_JUNCTION_RGB`] at [`ROAD_JUNCTION_ALPHA`].
fn draw_junctions(
    image_set: &RoadImageSet<'_>,
    junctions: &[RoadJunction],
    master: &mut RoadCanvas,
) {
    for junction in junctions {
        if junction.degree >= f64::from(ROAD_JUNCTION_MIN_DEGREE) {
            let [centre_x, centre_y] = image_set.world_to_screen(junction.world_position);
            master.draw_disc(
                centre_x,
                centre_y,
                ROAD_JUNCTION_RADIUS_PX,
                ROAD_JUNCTION_RGB,
                ROAD_JUNCTION_ALPHA,
            );
        }
    }
}

/// Writes `canvas` as the RGBA PNG at `path`.
fn write_rgba_image(path: &Path, size_px: u32, canvas: &RoadCanvas) -> Result<()> {
    write_png_rows(
        path,
        size_px,
        size_px,
        PngPixelLayout::Rgba8,
        |row_index, row| canvas.fill_rgba_row(row_index, row),
    )
}

/// The larger of `left` and `right`, or NaN when either is NaN.
fn maximum_or_nan(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.max(right)
    }
}

/// The size of the file at `path` in KB with one decimal, or `unknown size` when it cannot be
/// read.
fn file_size_text(path: &Path) -> String {
    std::fs::metadata(path).map_or_else(
        |_| "unknown size".to_string(),
        |metadata| format!("{:.1} KB", metadata.len() as f64 / 1024.0),
    )
}
