//! `world` — the world-export pipeline CLI: every stage that turns a Workbench export into the
//! committed terrain artifacts, and every gate that verifies one.

use std::process::ExitCode;

use std::path::PathBuf;

use crate::enfusion_pak::PakVfs;
use crate::world_export_pipeline::{
    chunk_partitioner, enfusion_texture_decoder, export_preparation, mathematical_verification,
    reclassify, roads_emit, topo,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "world", about = "World-export pipeline")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Report the topo file's sections and records, with a per-type histogram
    TopoStats {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Decode one supertexture cell: cell N -> raw RGBA on stdout, meta on stderr
    EddsCell { n: u32 },
    /// Export phase gate: refuse a phase above the registry importPhaseMax
    PhaseGate {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        phase: String,
    },
    /// Validate the committed export artifacts against their declared shapes
    ValidateExports,
    /// Census the exported world-object types and their classification status
    Census {
        #[arg(long)]
        terrain: String,
    },
    /// Verify the subregion spike's K1 gate
    SpikeK1 {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Census the subregion spike export
    SpikeCensus {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Verify the subregion spike's operations log
    SpikeOpsLog {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Copy a Workbench export profile into the scratch staging tree
    CopyExportProfile {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        full: bool,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        src: Option<String>,
        #[arg(long)]
        meta: Option<String>,
    },
    /// Build the SAP aerial cell index
    SapCatalog {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Repack a raw u16 heightfield into the DEM PNG
    RawU16DemPng {
        #[arg(long)]
        raster: std::path::PathBuf,
        #[arg(long)]
        meta: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// Partition the staged export into the committed chunk rows, catalogue and density grids
    BuildObjects {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        phase: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        patch_manifest: bool,
        #[arg(long)]
        ops_log: bool,
    },
    /// Re-derive density grids from committed objects (no staging/Workbench). Overwrites
    /// objects/density/*.bin at the current DENSITY_CELL_M with a canopy-blurred tree channel.
    Redensify {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Regenerate the golden S13 density fixture (bin + expectedCorners) at the current
    /// DENSITY_CELL_M. Run after a cell-size change so `cargo xtask ci schema-validate` (S13) stays green.
    GenDensityFixture,
    /// Rebuild the catalogue's classification lane from COMMITTED artifacts + the current
    /// prefab-classify.json. No Workbench, no staging, no game install. Default is a read-only
    /// drift check that exits 1 when a rule edit has gone latent; `--write` applies it.
    Reclassify {
        #[arg(long, default_value = "everon")]
        terrain: String,
        /// Apply the rebuild instead of only reporting drift.
        #[arg(long)]
        write: bool,
        /// Write under this base instead of assets_v2/terrains/<terrain> (relative = repo root).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Run the mathematical phase gate: G1-G12 + P-gates + D/F + E6 determinism
    VerifyPhase {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        phase: String,
    },
    /// Build the road network from the topo file. Dual emission: writes
    /// `objects/roads.json.gz` and `roads/road_network.rkyv`.
    BuildRoads {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        ops_log: bool,
    },
    /// Re-emit `roads/road_network.rkyv` from the committed `objects/roads.json.gz`.
    ///
    /// The archive half of `build-roads`, without the `.pak` VFS the topo decode needs — so the
    /// binary lane is reproducible from the repo alone rather than only on an export box.
    RoadsRkyv {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

pub fn entrypoint() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("world: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::TopoStats { terrain } => {
            let vfs = PakVfs::open_default()?;
            let t = topo::decode_topo(&vfs, &terrain)?;
            println!(
                "[topo] {terrain}: {} sections × {} records, consumed {}/{} bytes",
                t.section_count, t.per_section, t.consumed, t.bytes
            );
            let mut hist: std::collections::BTreeMap<u8, (u64, u64)> =
                std::collections::BTreeMap::new();
            for r in &t.records {
                let e = hist.entry(r.rec_type).or_default();
                e.0 += 1;
                e.1 += (r.verts.len() / 2) as u64;
            }
            for (ty, (n, verts)) in hist {
                println!("[topo]   type {ty}: {n} records, {verts} vertices");
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::BuildObjects {
            terrain,
            phase,
            out,
            patch_manifest,
            ops_log,
        } => {
            chunk_partitioner::build_world_objects(
                &terrain,
                &phase,
                out.as_deref(),
                patch_manifest,
                ops_log,
            )?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Redensify { terrain } => {
            chunk_partitioner::redensify_from_committed(&terrain)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::GenDensityFixture => {
            chunk_partitioner::gen_density_fixture()?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Reclassify {
            terrain,
            write,
            out,
        } => {
            let mode = if write {
                reclassify::Mode::Write
            } else {
                reclassify::Mode::Check
            };
            let base = reclassify::resolve_out_base(out.as_deref());
            Ok(ExitCode::from(reclassify::reclassify_terrain(
                &terrain,
                mode,
                base.as_deref(),
            )?))
        }
        Cmd::VerifyPhase { terrain, phase } => Ok(ExitCode::from(
            mathematical_verification::verify_phase(&terrain, &phase)?,
        )),
        Cmd::BuildRoads {
            terrain,
            out,
            ops_log,
        } => {
            chunk_partitioner::build_roads_from_topo(&terrain, out.as_deref(), ops_log)?;
            // Dual emission. It runs AFTER the JSON write and reads the file that write
            // just produced, so the archive is the JSON's centrelined twin by construction — see
            // `roads_emit`'s module docs. Both paths are printed because an operator who only
            // sees one of them cannot tell which lane a stale asset came from.
            let dir = roads_emit::resolve_terrain_dir(&terrain, out.as_deref());
            let (rkyv, bytes) = roads_emit::emit_road_network(&dir)?;
            println!(
                "build-roads: {} + {} ({bytes} bytes)",
                dir.join(roads_emit::ROADS_GZ).display(),
                rkyv.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::RoadsRkyv { terrain, out } => Ok(ExitCode::from(roads_emit::emit_road_network_cli(
            &terrain,
            out.as_deref(),
        )?)),
        Cmd::PhaseGate { terrain, phase } => Ok(ExitCode::from(
            mathematical_verification::phase_gate(&terrain, &phase)?,
        )),
        Cmd::ValidateExports => Ok(ExitCode::from(
            export_preparation::validate_export_artifacts()?,
        )),
        Cmd::Census { terrain } => Ok(ExitCode::from(export_preparation::census_types(&terrain)?)),
        Cmd::SpikeK1 { terrain } => Ok(ExitCode::from(export_preparation::verify_spike_k1(
            &terrain,
        )?)),
        Cmd::SpikeCensus { terrain } => {
            Ok(ExitCode::from(export_preparation::census_spike(&terrain)?))
        }
        Cmd::SpikeOpsLog { terrain } => Ok(ExitCode::from(
            export_preparation::verify_spike_ops_log(&terrain)?,
        )),
        Cmd::CopyExportProfile {
            terrain,
            full,
            profile,
            src,
            meta,
        } => Ok(ExitCode::from(
            export_preparation::copy_world_export_profile(&terrain, full, profile, src, meta)?,
        )),
        Cmd::SapCatalog { terrain } => Ok(ExitCode::from(export_preparation::catalog_sap_cells(
            &terrain,
        )?)),
        Cmd::RawU16DemPng { raster, meta, out } => Ok(ExitCode::from(
            export_preparation::raw_u16_to_dem_png(&raster, &meta, &out)?,
        )),
        Cmd::EddsCell { n } => {
            use std::io::Write as _;
            let vfs = PakVfs::open_default()?;
            let c = enfusion_texture_decoder::decode_cell_rgba(&vfs, n)?;
            eprintln!(
                "{}",
                serde_json::json!({ "cell": n, "side": c.side, "dxgi": c.dxgi, "mipCount": c.mip_count, "rgbaBytes": c.rgba.len() })
            );
            std::io::stdout().write_all(&c.rgba)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
