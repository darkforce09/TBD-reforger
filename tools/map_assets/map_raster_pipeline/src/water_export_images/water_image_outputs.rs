//! The statistics and the five PNG images of a water raster.
//!
//! **Role:** [`WaterStatistics`] counts the samples of each water class and the depth range of
//! the water; [`write_water_images`] writes the images an output selection asks for: the
//! transparent bathymetry (`-bathymetry.png`, RGBA), the dark bathymetry with sea contours
//! (`-bathymetry-dark.png`, RGB), the 16-bit depth (`-depth-16bit.png`), the class mask
//! (`-mask.png`) and the block-averaged preview (`-preview.png`, RGB).
//! **Position:** the last step of the lane's `run`; colours come from `water_bodies`' bathymetry
//! palette and every image streams through the crate's `write_png_rows`.
//! **Signals & state:** none held; reads the caller's [`WaterRaster`], writes files.
//! **Invariants:** every image is north-up: image row `r` shows raster row `height − 1 − r`; a
//! sample of class 0 is land and every other class is water; depth in metres is decimetres × 0.1;
//! land is transparent black on the bathymetry, [`DARK_LAND_RGB`] on the dark image and the
//! preview, and 0 on the depth image; a dark or preview channel is `round(colour × contour)`, and
//! every rounded channel here is non-negative, so [`f64::round`] rounds a tie up exactly as ties
//! toward +∞ would; the preview averages square blocks of `round(width / min(1600, width))`
//! samples, a block's class being river over lake over sea, and blends a sea block with dark land
//! by the share of the block that is not water (samples past the raster's edge count as land).

use std::path::Path;

use water_bodies::prelude::{
    DARK_LAND_RGB, WaterClass, bathymetry_rgb, contour_multiplier, palette_class_for_mask_code,
};

use super::water_raster::WaterRaster;
use crate::error::Result;
use crate::image_operations::png_writing::{PngPixelLayout, write_png_rows};

/// The largest side of the preview image, in pixels.
const PREVIEW_MAXIMUM_SIDE_PX: f64 = 1600.0;

/// Which of the five images to write.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WaterImageSelection {
    /// The transparent bathymetry (`-bathymetry.png`).
    pub(super) bathymetry: bool,
    /// The dark bathymetry with contours (`-bathymetry-dark.png`).
    pub(super) dark: bool,
    /// The 16-bit depth (`-depth-16bit.png`).
    pub(super) depth16: bool,
    /// The class mask (`-mask.png`).
    pub(super) mask: bool,
    /// The preview (`-preview.png`).
    pub(super) preview: bool,
}

/// The sample counts of each water class and the depth range of the water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WaterStatistics {
    /// Class 0 samples.
    pub(super) land: usize,
    /// Class 1 samples.
    pub(super) ocean: usize,
    /// Class 2 samples.
    pub(super) lake_or_pond: usize,
    /// Class 3 samples.
    pub(super) river: usize,
    /// The shallowest water depth in decimetres (0 when there is no water).
    pub(super) minimum_depth_decimetres: u16,
    /// The deepest water depth in decimetres.
    pub(super) maximum_depth_decimetres: u16,
}

impl WaterStatistics {
    /// The statistics of `raster`; a water sample of a class above 3 counts toward the depth
    /// range only.
    pub(super) fn of(raster: &WaterRaster) -> Self {
        let mut statistics = Self {
            land: 0,
            ocean: 0,
            lake_or_pond: 0,
            river: 0,
            minimum_depth_decimetres: u16::MAX,
            maximum_depth_decimetres: 0,
        };
        for (&class, &depth) in raster.mask.iter().zip(&raster.depth_decimetres) {
            match class {
                0 => {
                    statistics.land += 1;
                    continue;
                }
                1 => statistics.ocean += 1,
                2 => statistics.lake_or_pond += 1,
                3 => statistics.river += 1,
                _ => {}
            }
            statistics.minimum_depth_decimetres = statistics.minimum_depth_decimetres.min(depth);
            statistics.maximum_depth_decimetres = statistics.maximum_depth_decimetres.max(depth);
        }
        if statistics.minimum_depth_decimetres > statistics.maximum_depth_decimetres {
            statistics.minimum_depth_decimetres = 0;
        }
        statistics
    }
}

/// Writes the images `selection` asks for into `out_dir`, each named `<prefix><suffix>.png`, and
/// prints each path written.
pub(super) fn write_water_images(
    raster: &WaterRaster,
    selection: WaterImageSelection,
    out_dir: &Path,
    prefix: &str,
) -> Result<()> {
    let width = u32::try_from(raster.grid.width)?;
    let height = u32::try_from(raster.grid.height)?;
    let rows = raster.grid.height;
    let row_samples = |image_row: usize| {
        let start = raster.index(0, rows - 1 - image_row);
        let end = start + raster.grid.width;
        (
            &raster.mask[start..end],
            &raster.depth_decimetres[start..end],
        )
    };
    let path_of = |suffix: &str| out_dir.join(format!("{prefix}{suffix}.png"));

    if selection.bathymetry {
        let path = path_of("-bathymetry");
        write_png_rows(
            &path,
            width,
            height,
            PngPixelLayout::Rgba8,
            |image_row, row| {
                let (classes, depths) = row_samples(image_row);
                for ((pixel, &class), &depth) in row.chunks_exact_mut(4).zip(classes).zip(depths) {
                    if class == 0 {
                        pixel.copy_from_slice(&[0, 0, 0, 0]);
                    } else {
                        let rgb =
                            bathymetry_rgb(depth_m(depth), palette_class_for_mask_code(class));
                        pixel.copy_from_slice(&[rgb[0], rgb[1], rgb[2], u8::MAX]);
                    }
                }
            },
        )?;
        println!("Wrote {}", path.display());
    }
    if selection.dark {
        let path = path_of("-bathymetry-dark");
        write_png_rows(
            &path,
            width,
            height,
            PngPixelLayout::Rgb8,
            |image_row, row| {
                let (classes, depths) = row_samples(image_row);
                for ((pixel, &class), &depth) in row.chunks_exact_mut(3).zip(classes).zip(depths) {
                    let rgb = if class == 0 {
                        DARK_LAND_RGB
                    } else {
                        contoured_rgb(depth_m(depth), palette_class_for_mask_code(class))
                    };
                    pixel.copy_from_slice(&rgb);
                }
            },
        )?;
        println!("Wrote {}", path.display());
    }
    if selection.depth16 {
        let path = path_of("-depth-16bit");
        write_png_rows(
            &path,
            width,
            height,
            PngPixelLayout::Gray16,
            |image_row, row| {
                let (classes, depths) = row_samples(image_row);
                for ((pixel, &class), &depth) in row.chunks_exact_mut(2).zip(classes).zip(depths) {
                    let value = if class == 0 { 0 } else { depth };
                    pixel.copy_from_slice(&value.to_be_bytes());
                }
            },
        )?;
        println!("Wrote {}", path.display());
    }
    if selection.mask {
        let path = path_of("-mask");
        write_png_rows(
            &path,
            width,
            height,
            PngPixelLayout::Gray8,
            |image_row, row| {
                row.copy_from_slice(row_samples(image_row).0);
            },
        )?;
        println!("Wrote {}", path.display());
    }
    if selection.preview {
        let path = path_of("-preview");
        write_preview(raster, &path)?;
        println!("Wrote {}", path.display());
    }
    Ok(())
}

/// Depth in metres of a depth in decimetres.
fn depth_m(depth_decimetres: u16) -> f64 {
    f64::from(depth_decimetres) * 0.1
}

/// The bathymetry colour of `depth_m` of `class`, each channel scaled by the contour factor and
/// rounded (non-negative, so [`f64::round`] is exact here).
fn contoured_rgb(depth_m: f64, class: WaterClass) -> [u8; 3] {
    let rgb = bathymetry_rgb(depth_m, class);
    let contour = contour_multiplier(depth_m, class);
    rgb.map(|channel| (f64::from(channel) * contour).round() as u8)
}

/// Writes the block-averaged preview of `raster` at `path`; see the module invariants.
fn write_preview(raster: &WaterRaster, path: &Path) -> Result<()> {
    let (width, height) = (raster.grid.width, raster.grid.height);
    // Every quotient below is positive, so `f64::round` rounds exactly as ties toward +∞ would.
    let width_f = width as f64;
    let scale = (width_f / PREVIEW_MAXIMUM_SIDE_PX.min(width_f))
        .round()
        .max(1.0);
    let preview_width = (width_f / scale).round();
    let preview_height = (height as f64 / scale).round();
    // Whole and positive: `scale` is at least 1 and divides at most a u32 extent.
    let block = scale as usize;
    let block_area = scale * scale;
    write_png_rows(
        path,
        u32::try_from(preview_width as u64)?,
        u32::try_from(preview_height as u64)?,
        PngPixelLayout::Rgb8,
        |preview_row, row| {
            for (preview_column, pixel) in row.chunks_exact_mut(3).enumerate() {
                let mut depth_sum = 0_u64;
                let mut water = 0_usize;
                let mut river = 0_usize;
                let mut lake = 0_usize;
                for image_row in (preview_row * block..(preview_row + 1) * block)
                    .take_while(|&image_row| image_row < height)
                {
                    let raster_row = height - 1 - image_row;
                    for column in (preview_column * block..(preview_column + 1) * block)
                        .take_while(|&column| column < width)
                    {
                        let index = raster.index(column, raster_row);
                        let class = raster.mask[index];
                        if class == 0 {
                            continue;
                        }
                        water += 1;
                        depth_sum += u64::from(raster.depth_decimetres[index]);
                        match class {
                            3 => river += 1,
                            2 => lake += 1,
                            _ => {}
                        }
                    }
                }
                let rgb = if water == 0 {
                    DARK_LAND_RGB
                } else {
                    let class = if river > 0 {
                        WaterClass::River
                    } else if lake > 0 {
                        WaterClass::LakeOrPond
                    } else {
                        WaterClass::Ocean
                    };
                    let average_depth_m = (depth_sum as f64 / water as f64) * 0.1;
                    if class == WaterClass::Ocean {
                        let land_weight = (block_area - water as f64) / block_area;
                        blended_sea_rgb(average_depth_m, land_weight)
                    } else {
                        contoured_rgb(average_depth_m, class)
                    }
                };
                pixel.copy_from_slice(&rgb);
            }
        },
    )
}

/// The contoured sea colour of `depth_m` blended with dark land by `land_weight` (0 all water,
/// below 1 always), each channel rounded (non-negative, so [`f64::round`] is exact here).
fn blended_sea_rgb(depth_m: f64, land_weight: f64) -> [u8; 3] {
    let rgb = bathymetry_rgb(depth_m, WaterClass::Ocean);
    let contour = contour_multiplier(depth_m, WaterClass::Ocean);
    let mut blended = [0_u8; 3];
    for ((out, &channel), &land) in blended.iter_mut().zip(&rgb).zip(&DARK_LAND_RGB) {
        let value =
            (f64::from(channel) * contour) * (1.0 - land_weight) + f64::from(land) * land_weight;
        *out = value.round() as u8;
    }
    blended
}
