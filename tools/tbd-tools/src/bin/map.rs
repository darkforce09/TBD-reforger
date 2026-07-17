//! `map` — the T-165.9 map-asset image pipeline CLI (ports of the scripts/map-assets image
//! lane). Exit codes mirror the Node scripts.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tbd_tools::map::{carto, glyphs, labels, sap, unified, water};

#[derive(Parser)]
#[command(name = "map", about = "T-090 map-asset image pipeline (Rust)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// verify-unified-satellite.mjs port
    VerifyUnified {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// verify-tile-pyramid.mjs port (VIEW=map via --view-map; EXPECT_LOSSLESS=1 via --expect-lossless)
    VerifyPyramid {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        view_map: bool,
        #[arg(long)]
        expect_lossless: bool,
    },
    /// build-glyph-atlas.mjs port (SVG → lossless-WebP atlas + Deck mapping)
    BuildGlyphAtlas,
    /// build-landcover-mask.mjs port
    BuildLandcover {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// build-map-cartographic.mjs port (TGA + tints + water + resvg roads)
    BuildCartographic {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// build-tile-pyramid.sh port (XYZ WebP levels + full.webp)
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
    /// export-locations.mjs port
    ExportLocations {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        src: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// export-height-labels.mjs port (native core restore — wasm pkg is gone)
    ExportHeightLabels {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// map-water step: drop the waterComposite meta block (was a `node -e` one-liner)
    ResetWaterMeta {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// map-water step: manifest unified.bytes = bundle size (was a `node -e` one-liner)
    PatchUnifiedBytes {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// map-cartographic step: tiles.map source/encoding patch (was a `node -e` one-liner)
    PatchMapTilesMeta {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// verify-t152-cartographic.mjs port (program-wide aggregator)
    VerifyT152,
    /// analyze-water-sources.mjs port (inland-water classifier → mask + spike JSON)
    AnalyzeWater,
    /// composite-water-ortho.mjs port (ocean/inland tint over the SAP ortho, in place)
    CompositeWater,
    /// verify-sap-seams.mjs port
    VerifySapSeams {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// analyze-sap-seams.mjs port
    AnalyzeSapSeams {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// verify-sap-ortho.mjs port
    VerifySapOrtho {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// stitch-sap-ortho.mjs port (pak → 12800² north-up ortho + seam bridge)
    StitchSapOrtho {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// blend-sap-seams.mjs CLI port (bridge the existing ortho in place)
    BlendSapSeams {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// build-unified-satellite.mjs port
    BuildUnified {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long, default_value_t = 8192)]
        tile_threshold: usize,
    },
}

fn main() -> ExitCode {
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
        Cmd::VerifyUnified { terrain } => {
            Ok(ExitCode::from(unified::verify_unified_satellite(&terrain)?))
        }
        Cmd::VerifyPyramid {
            terrain,
            view_map,
            expect_lossless,
        } => Ok(ExitCode::from(unified::verify_tile_pyramid(
            &terrain,
            view_map,
            expect_lossless,
        )?)),
        Cmd::BuildGlyphAtlas => Ok(ExitCode::from(glyphs::build_glyph_atlas()?)),
        Cmd::BuildLandcover { terrain } => {
            Ok(ExitCode::from(carto::build_landcover_cli(&terrain)?))
        }
        Cmd::BuildCartographic { terrain } => {
            Ok(ExitCode::from(carto::build_map_cartographic(&terrain)?))
        }
        Cmd::BuildPyramid {
            input,
            out,
            minzoom,
            maxzoom,
            tilesize,
            quality,
            lossless,
            flip_v,
        } => Ok(ExitCode::from(carto::build_tile_pyramid(
            &input, &out, minzoom, maxzoom, tilesize, quality, lossless, flip_v,
        )?)),
        Cmd::ExportLocations {
            terrain,
            src,
            dry_run,
        } => Ok(ExitCode::from(labels::export_locations(
            &terrain, src, dry_run,
        )?)),
        Cmd::ExportHeightLabels { terrain } => {
            Ok(ExitCode::from(labels::export_height_labels(&terrain)?))
        }
        Cmd::ResetWaterMeta { terrain } => Ok(ExitCode::from(carto::reset_water_meta(&terrain)?)),
        Cmd::PatchUnifiedBytes { terrain } => {
            Ok(ExitCode::from(carto::patch_unified_bytes(&terrain)?))
        }
        Cmd::PatchMapTilesMeta { terrain } => {
            Ok(ExitCode::from(carto::patch_map_tiles_meta(&terrain)?))
        }
        Cmd::VerifyT152 => Ok(ExitCode::from(carto::verify_t152()?)),
        Cmd::AnalyzeWater => Ok(ExitCode::from(water::analyze_water_sources()?)),
        Cmd::CompositeWater => Ok(ExitCode::from(water::composite_water_ortho()?)),
        Cmd::VerifySapSeams { terrain } => Ok(ExitCode::from(sap::verify_sap_seams(&terrain)?)),
        Cmd::AnalyzeSapSeams { terrain } => Ok(ExitCode::from(sap::analyze_sap_seams(&terrain)?)),
        Cmd::VerifySapOrtho { terrain } => Ok(ExitCode::from(sap::verify_sap_ortho(&terrain)?)),
        Cmd::StitchSapOrtho { terrain } => Ok(ExitCode::from(sap::stitch_sap_ortho(&terrain)?)),
        Cmd::BlendSapSeams { terrain } => Ok(ExitCode::from(sap::blend_sap_seams_cli(&terrain)?)),
        Cmd::BuildUnified {
            input,
            out,
            terrain,
            tile_threshold,
        } => Ok(ExitCode::from(unified::build_unified_satellite(
            &input,
            &out,
            &terrain,
            tile_threshold,
        )?)),
    }
}
