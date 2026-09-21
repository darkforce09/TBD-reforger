//! `map` — the map-asset image pipeline CLI: satellite and cartographic rasters, tile pyramids,
//! glyph atlases, map labels and the inland-water lane.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::map_raster_pipeline::{
    aerial_orthophoto, cartographic_rendering, glyphs, inland_water, inland_water_archive,
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
    /// inland-water export in `assets_v2/scratch/<terrain>/water`. `--terrain` takes a terrain id,
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
}

pub fn entrypoint() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("map: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
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
        Cmd::BuildGlyphAtlas => Ok(ExitCode::from(glyphs::build_glyph_atlas()?)),
        Cmd::BuildLandcover { terrain } => Ok(ExitCode::from(
            cartographic_rendering::build_landcover_cli(&terrain)?,
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
        Cmd::Water { terrain } => Ok(ExitCode::from(inland_water_archive::emit_water(&terrain)?)),
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
        Cmd::VerifySapSeams { terrain } => Ok(ExitCode::from(aerial_orthophoto::verify_sap_seams(
            &terrain,
        )?)),
        Cmd::AnalyzeSapSeams { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::analyze_sap_seams(&terrain)?,
        )),
        Cmd::VerifySapOrtho { terrain } => Ok(ExitCode::from(aerial_orthophoto::verify_sap_ortho(
            &terrain,
        )?)),
        Cmd::StitchSapOrtho { terrain } => Ok(ExitCode::from(aerial_orthophoto::stitch_sap_ortho(
            &terrain,
        )?)),
        Cmd::BlendSapSeams { terrain } => Ok(ExitCode::from(
            aerial_orthophoto::blend_sap_seams_cli(&terrain)?,
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
    }
}
