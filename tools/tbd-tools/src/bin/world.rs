//! `world` — the T-165.8 world-export pipeline CLI (ports of the scripts/map-assets export
//! lane). Subcommands land through the slice; exit codes mirror the Node scripts.

use std::process::ExitCode;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tbd_tools::world::{aux, build, edds, gates, pak::PakVfs, topo};

#[derive(Parser)]
#[command(name = "world", about = "T-090 world-export pipeline (Rust)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// decode-topo.mjs CLI: section/record stats + per-type histogram
    TopoStats {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// decode-edds.mjs CLI: cell N -> raw RGBA on stdout, meta on stderr
    EddsCell { n: u32 },
    /// export-terrain.sh phase gate (registry importPhaseMax check)
    PhaseGate {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        phase: String,
    },
    /// validate-export-artifacts.mjs port (make map-export-validate)
    ValidateExports,
    /// census-types.mjs port (make map-census)
    Census {
        #[arg(long)]
        terrain: String,
    },
    /// verify-spike-k1.mjs port
    SpikeK1 {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// census-spike.mjs port
    SpikeCensus {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// verify-spike-ops-log.mjs port
    SpikeOpsLog {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// copy-world-export-profile.mjs port
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
    /// catalog-sap-cells.mjs port (T-090.1.2 SAP cell index)
    SapCatalog {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// raw-u16-to-dem-png.mjs port (T-091.0)
    RawU16DemPng {
        #[arg(long)]
        raster: std::path::PathBuf,
        #[arg(long)]
        meta: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// build-world-objects.mjs port
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
    /// verify-phase.mjs port: G1-G12 + P-gates + D/F + E6 determinism
    VerifyPhase {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        phase: String,
    },
    /// build-roads-from-topo.mjs port
    BuildRoads {
        #[arg(long)]
        terrain: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        ops_log: bool,
    },
}

fn main() -> ExitCode {
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
            build::build_world_objects(&terrain, &phase, out.as_deref(), patch_manifest, ops_log)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::VerifyPhase { terrain, phase } => {
            Ok(ExitCode::from(gates::verify_phase(&terrain, &phase)?))
        }
        Cmd::BuildRoads {
            terrain,
            out,
            ops_log,
        } => {
            build::build_roads_from_topo(&terrain, out.as_deref(), ops_log)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::PhaseGate { terrain, phase } => Ok(ExitCode::from(aux::phase_gate(&terrain, &phase)?)),
        Cmd::ValidateExports => Ok(ExitCode::from(aux::validate_export_artifacts()?)),
        Cmd::Census { terrain } => Ok(ExitCode::from(aux::census_types(&terrain)?)),
        Cmd::SpikeK1 { terrain } => Ok(ExitCode::from(aux::verify_spike_k1(&terrain)?)),
        Cmd::SpikeCensus { terrain } => Ok(ExitCode::from(aux::census_spike(&terrain)?)),
        Cmd::SpikeOpsLog { terrain } => Ok(ExitCode::from(aux::verify_spike_ops_log(&terrain)?)),
        Cmd::CopyExportProfile {
            terrain,
            full,
            profile,
            src,
            meta,
        } => Ok(ExitCode::from(aux::copy_world_export_profile(
            &terrain, full, profile, src, meta,
        )?)),
        Cmd::SapCatalog { terrain } => Ok(ExitCode::from(aux::catalog_sap_cells(&terrain)?)),
        Cmd::RawU16DemPng { raster, meta, out } => Ok(ExitCode::from(aux::raw_u16_to_dem_png(
            &raster, &meta, &out,
        )?)),
        Cmd::EddsCell { n } => {
            use std::io::Write as _;
            let vfs = PakVfs::open_default()?;
            let c = edds::decode_cell_rgba(&vfs, n)?;
            eprintln!(
                "{}",
                serde_json::json!({ "cell": n, "side": c.side, "dxgi": c.dxgi, "mipCount": c.mip_count, "rgbaBytes": c.rgba.len() })
            );
            std::io::stdout().write_all(&c.rgba)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}
