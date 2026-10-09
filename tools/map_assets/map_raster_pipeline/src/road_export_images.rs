//! The road export image lane: the Workbench road export drawn as PNG images.
//!
//! **Role:** `map road-images`, which strokes the road layers of a Workbench road export onto a
//! transparent master image, a dark master image and one image per road class. [`run`] checks the
//! folders, then hands the loaded export (`road_layer_loading`) to the drawing and writing
//! (`road_image_outputs`) over the RGBA canvas (`road_canvas`).
//! **Position:** a lane of the `map` binary, called by the command line with a
//! [`RoadImageOptions`]; over `road_network`'s export image styling, `grid_rasterization`'s disc
//! stamps and the crate's streaming PNG writer (`image_operations::png_writing`).
//! **Signals & state:** none held; one call reads the export, draws its canvases and writes them.
//! **Invariants:** the lane writes only under its output folder; the canvas is square, one pixel
//! per `world size / size` metres; a malformed export file is warned about and drawn as empty, so
//! the masters are always written.

mod road_canvas;
mod road_image_outputs;
mod road_layer_loading;

use std::path::PathBuf;

use crate::error::{Result, ResultExt, bail};

/// The world size, in metres, of an export whose `roads_meta.json` names none.
const DEFAULT_WORLD_SIZE_M: f64 = 12800.0;

/// What one `map road-images` run reads, draws and writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoadImageOptions {
    /// The Workbench road export folder: `roads_meta.json` and the six layer files.
    pub(crate) roads_dir: PathBuf,
    /// Where the images go; `<roads_dir>/images` when absent.
    pub(crate) out_dir: Option<PathBuf>,
    /// The width and height of every image, in pixels.
    pub(crate) size_px: u32,
    /// The terrain name the two master images are prefixed with.
    pub(crate) terrain: String,
    /// Whether the transparent and dark masters mark the junctions of three or more roads.
    pub(crate) show_junctions: bool,
}

/// Runs `map road-images`: reads the export under [`RoadImageOptions::roads_dir`] and writes
/// `layer-<file stem>.png` for every layer that has segments, then `<terrain>-roads-transparent.png`
/// and `<terrain>-roads-dark.png`, into the output folder (created when missing). Returns exit
/// code 0; a missing roads folder, a zero size or a failed write is an [`crate::error::Error`].
pub(crate) fn run(options: &RoadImageOptions) -> Result<u8> {
    let roads_dir = &options.roads_dir;
    let out_dir = options
        .out_dir
        .clone()
        .unwrap_or_else(|| roads_dir.join("images"));
    println!("=== map road-images ===");
    println!("Source roads dir: {}", roads_dir.display());
    println!("Target out dir  : {}", out_dir.display());
    println!(
        "Resolution      : {} x {} px",
        options.size_px, options.size_px
    );
    println!("Show junctions  : {}", options.show_junctions);
    if !roads_dir.is_dir() {
        bail!("roads folder not found: {}", roads_dir.display());
    }
    if options.size_px == 0 {
        bail!("--size must be at least 1 pixel");
    }
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("create the output folder {}", out_dir.display()))?;

    let meta = road_layer_loading::load_road_export_meta(roads_dir, DEFAULT_WORLD_SIZE_M);
    let layers = road_layer_loading::load_road_layers(roads_dir);
    let total_segments: usize = layers.iter().map(|layer| layer.listed_segment_count).sum();
    println!("\nTotal continuous routes to render: {total_segments}");

    let image_set = road_image_outputs::RoadImageSet {
        out_dir: &out_dir,
        size_px: options.size_px,
        world_size_m: meta.world_size_m,
        terrain: &options.terrain,
    };
    let junctions: &[road_layer_loading::RoadJunction] = if options.show_junctions {
        &meta.junctions
    } else {
        &[]
    };
    road_image_outputs::draw_and_write_road_images(&image_set, &layers, junctions)?;
    println!("=== map road-images complete ===");
    Ok(0)
}

#[cfg(test)]
#[path = "tests/road_export_images_tests.rs"]
mod tests;
