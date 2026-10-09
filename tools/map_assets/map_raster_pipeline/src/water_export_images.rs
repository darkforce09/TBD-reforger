//! The water export image lane: the Workbench water export drawn as PNG images.
//!
//! **Role:** `map water-images` ([`run`] over [`WaterImageOptions`]): finds a Workbench water
//! export, decodes its ASCII class mask and depth grids, draws its lakes, ponds and rivers over
//! them, and writes the bathymetry, dark bathymetry, 16-bit depth, class mask and preview PNGs
//! the [`WaterImageMode`] selects; [`parse_region_of_interest`] reads `--roi`.
//! **Position:** a lane of the `map` binary (`command_line.rs` builds the options) over
//! `water_bodies`' bathymetry palette, `grid_rasterization`'s rounding, spline and scanline
//! arithmetic, `terrain_elevation`'s 16-bit PNG decoder and the crate's streaming PNG writer; its
//! submodules sit in `water_export_images/`.
//! **Signals & state:** none held; each run reads the export, holds one mask and one depth grid
//! of the image size, and writes its images.
//! **Invariants:** the lane writes only into its output folder; the export grids are read only
//! when the images cover the export's own grid (no region, the meta's pixel size), otherwise the
//! raster starts as land; the vectors draw over the grids unless `--no-vector-enhance` is given;
//! the same export and options give the same pixels.

mod dem_sampling;
mod export_location;
mod json_field_reading;
mod polygon_rasterization;
mod raster_geometry;
mod river_rasterization;
mod vector_rasterization;
mod water_grid_decoding;
mod water_image_outputs;
mod water_raster;

use std::path::PathBuf;
use std::time::Instant;

use serde_json::Value;

use crate::error::{Result, ResultExt};
use export_location::{create_output_directory, locate_water_export, output_prefix};
use raster_geometry::water_raster_geometry;
use vector_rasterization::rasterize_water_vectors;
use water_grid_decoding::{decode_depth_grid, decode_mask_grid};
use water_image_outputs::{WaterImageSelection, WaterStatistics, write_water_images};
use water_raster::WaterRaster;

/// The options of `map water-images`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WaterImageOptions {
    /// The Workbench export folder, or a folder above it; see `export_location.rs` for the
    /// folders searched under it.
    pub(crate) export_dir: PathBuf,
    /// The image folder; `images` under the water folder when not given.
    pub(crate) out_dir: Option<PathBuf>,
    /// Which images to write.
    pub(crate) mode: WaterImageMode,
    /// The terrain name: a search folder under the export folder and the image name prefix.
    pub(crate) terrain: String,
    /// A 16-bit grey elevation PNG (2 m per sample) that deepens lakes and ponds to the water
    /// column above the terrain; without it they take their exported depths.
    pub(crate) dem_path: Option<PathBuf>,
    /// Accepted for option compatibility and has no effect: the inland files are chosen by the
    /// metadata file the export folder holds.
    pub(crate) inland_only: bool,
    /// The image resolution in metres per pixel; a value that is not positive (or NaN) counts as
    /// not given, leaving the meta's resolution.
    pub(crate) resolution_m_per_px: Option<f64>,
    /// A world region `[min_x, min_z, max_x, max_z]` to draw instead of the whole world; only
    /// the vectors are drawn over it.
    pub(crate) region_of_interest: Option<[f64; 4]>,
    /// Whether the lake, pond and river vectors are drawn (`false` with `--no-vector-enhance`).
    pub(crate) vector_enhance: bool,
}

/// Which images `map water-images` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum WaterImageMode {
    /// The transparent bathymetry (`-bathymetry.png`).
    Water,
    /// The transparent bathymetry (`-bathymetry.png`), as `water`.
    Bathymetry,
    /// The dark bathymetry with sea contours (`-bathymetry-dark.png`).
    Dark,
    /// The 16-bit depth in decimetres (`-depth-16bit.png`).
    #[value(name = "depth16")]
    Depth16,
    /// The water class mask (`-mask.png`).
    Mask,
    /// The block-averaged preview (`-preview.png`).
    Preview,
    /// All five images.
    All,
}

impl WaterImageMode {
    /// The images this mode writes.
    fn selection(self) -> WaterImageSelection {
        let all = self == WaterImageMode::All;
        WaterImageSelection {
            bathymetry: all || matches!(self, WaterImageMode::Water | WaterImageMode::Bathymetry),
            dark: all || self == WaterImageMode::Dark,
            depth16: all || self == WaterImageMode::Depth16,
            mask: all || self == WaterImageMode::Mask,
            preview: all || self == WaterImageMode::Preview,
        }
    }
}

/// Parses `--roi`: four comma-separated finite numbers `min_x,min_z,max_x,max_z`, each trimmed of
/// surrounding white space.
pub(crate) fn parse_region_of_interest(text: &str) -> std::result::Result<[f64; 4], String> {
    let values = text
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("`{}` is not a finite number", part.trim()))
        })
        .collect::<std::result::Result<Vec<f64>, String>>()?;
    <[f64; 4]>::try_from(values).map_err(|values| {
        format!(
            "expected four values min_x,min_z,max_x,max_z, found {}",
            values.len()
        )
    })
}

/// Runs `map water-images`: locates the export, builds the raster and writes the images
/// `options.mode` selects; returns exit code 0. Refuses an export with no metadata file, an
/// image size that is not a whole number of pixels, and an input that does not read or parse.
pub(crate) fn run(options: &WaterImageOptions) -> Result<u8> {
    println!("Water export images: {}", options.export_dir.display());
    if options.inland_only {
        println!("--inland-only has no effect: the export's metadata file selects its files");
    }
    let files = locate_water_export(&options.export_dir, &options.terrain)?;
    println!("Water data folder: {}", files.water_dir.display());
    let out_dir = create_output_directory(options.out_dir.as_deref(), &files.water_dir)?;
    println!("Image folder: {}", out_dir.display());
    let meta_text = std::fs::read_to_string(&files.meta_path)
        .with_context(|| format!("read {}", files.meta_path.display()))?;
    let meta: Value = serde_json::from_str(&meta_text)
        .with_context(|| format!("parse {}", files.meta_path.display()))?;

    let resolution = options
        .resolution_m_per_px
        .filter(|resolution| *resolution > 0.0);
    let geometry = water_raster_geometry(&meta, resolution, options.region_of_interest)?;
    let prefix = format!(
        "{}{}",
        output_prefix(&options.terrain, files.is_inland),
        geometry.region_suffix.as_deref().unwrap_or_default()
    );
    println!(
        "Image grid: {}x{} px at {} m/px",
        geometry.grid.width, geometry.grid.height, geometry.metres_per_pixel
    );

    let mut raster = WaterRaster::new(geometry.grid);
    if let (true, Some(mask_path), Some(depth_path)) = (
        geometry.is_export_grid,
        files.mask_path.as_ref(),
        files.depth_path.as_ref(),
    ) {
        let started = Instant::now();
        let mask_samples = decode_mask_grid(mask_path, &mut raster.mask)?;
        let depth_samples = decode_depth_grid(depth_path, &mut raster.depth_decimetres)?;
        println!(
            "Decoded the export grids ({mask_samples} mask and {depth_samples} depth samples) in {} ms",
            started.elapsed().as_millis()
        );
    }

    let has_vectors =
        files.lakes_path.is_some() || files.ponds_path.is_some() || files.rivers_path.is_some();
    if options.vector_enhance && has_vectors {
        let started = Instant::now();
        let counts = rasterize_water_vectors(&files, options.dem_path.as_deref(), &mut raster)?;
        println!(
            "Drew {} lakes, {} ponds and {} river segments in {} ms",
            counts.lakes,
            counts.ponds,
            counts.river_segments,
            started.elapsed().as_millis()
        );
    }

    let statistics = WaterStatistics::of(&raster);
    println!(
        "Samples: land {}, sea {}, lake or pond {}, river {}; water depth {:.1} m to {:.1} m",
        statistics.land,
        statistics.ocean,
        statistics.lake_or_pond,
        statistics.river,
        f64::from(statistics.minimum_depth_decimetres) * 0.1,
        f64::from(statistics.maximum_depth_decimetres) * 0.1
    );
    write_water_images(&raster, options.mode.selection(), &out_dir, &prefix)?;
    Ok(0)
}

#[cfg(test)]
#[path = "tests/water_export_images_tests.rs"]
mod tests;
