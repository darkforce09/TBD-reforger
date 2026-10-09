//! `map`, the map raster pipeline's command line: satellite and cartographic rasters, tile
//! pyramids, glyph atlases, map labels, the inland-water lane and the Workbench water and road
//! export images.
//!
//! **Role:** the clap command tree of the twenty-four subcommands and the dispatch of each to its
//! lane's entry function.
//! **Position:** the `map` binary of `developer_tools` calls [`entrypoint`]; the lanes of this
//! crate do the work and return the exit code.
//! **Signals & state:** none; parses the process arguments once.
//! **Invariants:** a lane's exit code is the process exit code; any failure prints
//! `map: <message>` with every cause underneath and exits 1; a clap usage error exits 2; the
//! export-image subcommands take their input folder exactly once, positionally or by option.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::error::{Result, ResultExt};
use crate::road_export_images::{self, RoadImageOptions};
use crate::water_export_images::{self, WaterImageMode, WaterImageOptions};
use crate::{
    aerial_orthophoto, cartographic_rendering, glyph_atlas, inland_water, inland_water_archive,
    map_label_archives, map_labels, satellite_archive, satellite_archive_container,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "map", about = "Map-asset image pipeline")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Verify the unified satellite bundle against its manifest
    VerifyUnified {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Verify a tile pyramid (the Map view via --view-map; lossless encoding via --expect-lossless)
    VerifyPyramid {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        view_map: bool,
        #[arg(long)]
        expect_lossless: bool,
    },
    /// Build the glyph atlas: SVG → lossless-WebP atlas + symbol mapping
    BuildGlyphAtlas,
    /// Build the land-cover mask from the stitched orthophoto
    BuildLandcover {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Build the cartographic ortho: TGA + tints + water + rendered roads
    BuildCartographic {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Build an XYZ WebP tile pyramid plus full.webp
    BuildPyramid {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 0)]
        minzoom: u32,
        #[arg(long, default_value_t = 5)]
        maxzoom: u32,
        #[arg(long, default_value_t = 256)]
        tilesize: usize,
        #[arg(long, default_value_t = 80.0)]
        quality: f32,
        #[arg(long)]
        lossless: bool,
        #[arg(long)]
        flip_v: bool,
    },
    /// Export the locations label set for a terrain
    ExportLocations {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        src: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Export the height-label set for a terrain
    ExportHeightLabels {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Emit `locations/map_labels.rkyv` from locations.json + height-labels.json +
    /// road-names.json (dual emission; the JSON files stay). `--terrain` takes a terrain id or a
    /// terrain directory.
    LabelsRkyv {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Emit `water/water_vectors.rkyv` + `water/bathymetry.tbd-bath` from the Workbench
    /// inland-water export in `assets/scratch/<terrain>/water`. `--terrain` takes a terrain id,
    /// or a directory whose export sits under its own `scratch/water`.
    Water {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// map-water step: drop the waterComposite meta block
    ResetWaterMeta {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// map-water step: set manifest unified.bytes to the bundle size
    PatchUnifiedBytes {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// map-cartographic step: patch the manifest tiles.map source and encoding
    PatchMapTilesMeta {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Program-wide cartographic aggregator: slice logs + the live sub-verifiers
    VerifyCartographic,
    /// Inland-water classifier: mask + source-spike JSON
    AnalyzeWater,
    /// Composite the ocean/inland tint over the stitched ortho, in place
    CompositeWater,
    /// Verify the stitched ortho carries no visible cell seams
    VerifySapSeams {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Measure the stitched ortho's cell seams and write the analysis artifact
    AnalyzeSapSeams {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Verify the stitched ortho against its metadata
    VerifySapOrtho {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Stitch the supertexture cells: pak → 12800² north-up ortho + seam bridge
    StitchSapOrtho {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Bridge the seams of the existing ortho, in place
    BlendSapSeams {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Build the unified satellite container from a source raster
    BuildUnified {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long, default_value_t = 8192)]
        tile_threshold: usize,
        /// TBDS container version: 2 (32-byte header + rkyv TbdSatIndexV2, the
        /// default) or 1 (the hand-packed JSON table `everon-sat.tbd-sat` is committed as).
        #[arg(long, default_value_t = satellite_archive_container::DEFAULT_CONTAINER_VERSION)]
        container_version: u16,
    },
    /// Draw a Workbench water export as PNG images: bathymetry, dark bathymetry, 16-bit depth,
    /// class mask and preview
    WaterImages {
        /// The Workbench export folder, or a folder above its water folder
        #[arg(
            value_name = "EXPORT_DIR",
            required_unless_present = "export_dir_option",
            conflicts_with = "export_dir_option"
        )]
        export_dir: Option<PathBuf>,
        /// The Workbench export folder, given as an option instead of positionally
        #[arg(long = "export-dir", value_name = "DIR")]
        export_dir_option: Option<PathBuf>,
        /// The image folder; `images` under the water folder when not given
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
        /// Which images to write
        #[arg(long, value_enum, ignore_case = true, default_value = "water")]
        mode: WaterImageMode,
        /// The terrain name: a folder searched under the export folder and the image name prefix
        #[arg(long, default_value = "everon")]
        terrain: String,
        /// A 16-bit grey elevation PNG at 2 m per sample that deepens lakes and ponds to the
        /// water column above the terrain
        #[arg(long, value_name = "PNG")]
        dem: Option<PathBuf>,
        /// Accepted and has no effect: the export's metadata file selects the inland files
        #[arg(long)]
        inland_only: bool,
        /// The image resolution in metres per pixel; zero or less keeps the export's resolution
        #[arg(long, value_name = "M_PER_PX", allow_negative_numbers = true)]
        res: Option<f64>,
        /// Draw only the world region MIN_X,MIN_Z,MAX_X,MAX_Z (metres); only the vectors are drawn
        #[arg(
            long,
            value_name = "MIN_X,MIN_Z,MAX_X,MAX_Z",
            allow_hyphen_values = true,
            value_parser = water_export_images::parse_region_of_interest
        )]
        roi: Option<[f64; 4]>,
        /// Skip drawing the lake, pond and river vectors over the export grids
        #[arg(long)]
        no_vector_enhance: bool,
    },
    /// Draw a Workbench road export as PNG images: transparent and dark masters and one layer per
    /// road class
    RoadImages {
        /// The Workbench road export folder: `roads_meta.json` and the six layer files
        #[arg(
            value_name = "ROADS_DIR",
            required_unless_present = "roads_dir_option",
            conflicts_with = "roads_dir_option"
        )]
        roads_dir: Option<PathBuf>,
        /// The Workbench road export folder, given as an option instead of positionally
        #[arg(long = "roads-dir", value_name = "DIR")]
        roads_dir_option: Option<PathBuf>,
        /// The image folder; `images` under the roads folder when not given
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
        /// The width and height of every image, in pixels
        #[arg(long, value_name = "PX", default_value_t = 2048)]
        size: u32,
        /// The terrain name the two master images are prefixed with
        #[arg(long, default_value = "everon")]
        terrain: String,
        /// Mark the junctions of three or more roads on the two master images
        #[arg(long)]
        show_junctions: bool,
    },
}

/// Runs the `map` command line: parses the process arguments, runs the subcommand and returns its
/// exit code; a failure prints `map: <message>` followed by every cause underneath it, each after
/// `: `, and exits 1.
pub fn entrypoint() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("map: {}", e.chain_text());
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::VerifyUnified { terrain } => Ok(ExitCode::from(
            satellite_archive::verify_unified_satellite(&terrain)?,
        )),
        Cmd::VerifyPyramid {
            terrain,
            view_map,
            expect_lossless,
        } => Ok(ExitCode::from(satellite_archive::verify_tile_pyramid(
            &terrain,
            view_map,
            expect_lossless,
        )?)),
        Cmd::BuildGlyphAtlas => Ok(ExitCode::from(glyph_atlas::build_glyph_atlas()?)),
        Cmd::BuildLandcover { terrain } => Ok(ExitCode::from(
            cartographic_rendering::build_landcover_command_line(&terrain)?,
        )),
        Cmd::BuildCartographic { terrain } => Ok(ExitCode::from(
            cartographic_rendering::build_map_cartographic(&terrain)?,
        )),
        Cmd::BuildPyramid {
            input,
            out,
            minzoom,
            maxzoom,
            tilesize,
            quality,
            lossless,
            flip_v,
        } => Ok(ExitCode::from(cartographic_rendering::build_tile_pyramid(
            &input, &out, minzoom, maxzoom, tilesize, quality, lossless, flip_v,
        )?)),
        Cmd::ExportLocations {
            terrain,
            src,
            dry_run,
        } => Ok(ExitCode::from(map_labels::export_locations(
            &terrain, src, dry_run,
        )?)),
        Cmd::ExportHeightLabels { terrain } => {
            Ok(ExitCode::from(map_labels::export_height_labels(&terrain)?))
        }
        Cmd::LabelsRkyv { terrain } => Ok(ExitCode::from(map_label_archives::emit_map_labels(
            &terrain,
        )?)),
        Cmd::Water { terrain } => Ok(ExitCode::from(inland_water_archive::emit_water_archive(
            &terrain,
        )?)),
        Cmd::ResetWaterMeta { terrain } => Ok(ExitCode::from(
            cartographic_rendering::reset_water_meta(&terrain)?,
        )),
        Cmd::PatchUnifiedBytes { terrain } => Ok(ExitCode::from(
            cartographic_rendering::patch_unified_bytes(&terrain)?,
        )),
        Cmd::PatchMapTilesMeta { terrain } => Ok(ExitCode::from(
            cartographic_rendering::patch_map_tiles_meta(&terrain)?,
        )),
        Cmd::VerifyCartographic => Ok(ExitCode::from(
            cartographic_rendering::verify_cartographic()?
        )),
        Cmd::AnalyzeWater => Ok(ExitCode::from(inland_water::analyze_water_sources()?)),
        Cmd::CompositeWater => Ok(ExitCode::from(inland_water::composite_water_ortho()?)),
        Cmd::VerifySapSeams { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::verify_supertexture_seams(&terrain)?,
        )),
        Cmd::AnalyzeSapSeams { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::analyze_supertexture_seams(&terrain)?,
        )),
        Cmd::VerifySapOrtho { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::verify_supertexture_orthophoto(&terrain)?,
        )),
        Cmd::StitchSapOrtho { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::stitch_supertexture_orthophoto(&terrain)?,
        )),
        Cmd::BlendSapSeams { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::blend_supertexture_seams_command_line(&terrain)?,
        )),
        Cmd::BuildUnified {
            input,
            out,
            terrain,
            tile_threshold,
            container_version,
        } => Ok(ExitCode::from(satellite_archive::build_unified_satellite(
            &input,
            &out,
            &terrain,
            tile_threshold,
            container_version,
        )?)),
        Cmd::WaterImages {
            export_dir,
            export_dir_option,
            out_dir,
            mode,
            terrain,
            dem,
            inland_only,
            res,
            roi,
            no_vector_enhance,
        } => Ok(ExitCode::from(water_export_images::run(
            &WaterImageOptions {
                export_dir: export_dir
                    .or(export_dir_option)
                    .context("water-images needs an export folder: EXPORT_DIR or --export-dir")?,
                out_dir,
                mode,
                terrain,
                dem_path: dem,
                inland_only,
                resolution_m_per_px: res,
                region_of_interest: roi,
                vector_enhance: !no_vector_enhance,
            },
        )?)),
        Cmd::RoadImages {
            roads_dir,
            roads_dir_option,
            out_dir,
            size,
            terrain,
            show_junctions,
        } => Ok(ExitCode::from(road_export_images::run(
            &RoadImageOptions {
                roads_dir: roads_dir
                    .or(roads_dir_option)
                    .context("road-images needs a roads folder: ROADS_DIR or --roads-dir")?,
                out_dir,
                size_px: size,
                terrain,
                show_junctions,
            },
        )?)),
    }
}
