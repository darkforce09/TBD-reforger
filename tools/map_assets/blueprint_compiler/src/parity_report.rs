//! `cargo xtask map parity-report`: the Workbench parity oracle replayed through a blueprint.
//!
//! **Role:** replays every observer and target pair of a parity file (engine `TraceMove` verdicts
//! the `EMCP_WB_TbdBlueprint` action `parity` records in the building's local frame, the frame
//! the blueprint uses, with glass panes excluded because vision passes glass) through
//! `BuildingBlueprint::annotate_sight_line`: the BVH raycast over the `.bvh` occlusion sidecar
//! decides clear or blocked and the blueprint names the hit; prints where the model and the
//! engine disagree. Usage: `--pairs <parity.json> --blueprint <blueprint.json> --sidecar
//! <file.bvh>`.
//! **Position:** called by the `cargo xtask map parity-report` adapter; [`ParityFile`] is also
//! read by the world line-of-sight tests of the map asset verification.
//! **Signals & state:** none; reads three files and prints.
//! **Invariants:** report only: the agreement number is the instrument, not a gate; the pinned
//! agreement lives in the `farmhouse_golden_parity_is_pinned` test of
//! the `blueprint_from_voxels` module.

use std::fs;
use std::path::PathBuf;

use crate::error::{Result, ResultExt};
use building_interiors::blueprint::structure::BuildingBlueprint;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;

/// A Workbench parity file: engine line-of-sight verdicts for observer and target pairs in one
/// building's local frame.
#[derive(serde::Deserialize)]
pub struct ParityFile {
    /// The building's slug, as its blueprint and sidecar are named.
    pub slug: String,
    /// `[ox, oy, oz, tx, ty, tz, engineClear]` per row: observer, target and the engine's verdict.
    pub pairs: Vec<(f64, f64, f64, f64, f64, f64, bool)>,
}

const USAGE: &str = "--pairs <json> --blueprint <json> --sidecar <file.bvh>";

/// Runs `parity-report` with the raw `args`: replays a Workbench parity file through a
/// blueprint and its sidecar and prints where the model and the engine disagree; returns the
/// exit code (0 the report printed, 1 an argument missing).
pub fn run(_root: &std::path::Path, args: &[String]) -> Result<u8> {
    let mut pairs_path: Option<PathBuf> = None;
    let mut bp_path: Option<PathBuf> = None;
    let mut sidecar_path: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--pairs" if i + 1 < args.len() => {
                pairs_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--blueprint" if i + 1 < args.len() => {
                bp_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--sidecar" if i + 1 < args.len() => {
                sidecar_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!("parity-report: unknown arg {other} (usage: {USAGE})");
                return Ok(1);
            }
        }
    }
    let (Some(pairs_path), Some(bp_path), Some(sidecar_path)) = (pairs_path, bp_path, sidecar_path)
    else {
        eprintln!("parity-report: --pairs, --blueprint and --sidecar are all required ({USAGE})");
        return Ok(1);
    };

    let parity: ParityFile = serde_json::from_str(
        &fs::read_to_string(&pairs_path)
            .with_context(|| format!("read {}", pairs_path.display()))?,
    )
    .context("parse parity JSON")?;
    let bp: BuildingBlueprint = serde_json::from_str(
        &fs::read_to_string(&bp_path).with_context(|| format!("read {}", bp_path.display()))?,
    )
    .context("parse blueprint JSON")?;
    let sidecar = BvhSidecar::parse(
        &fs::read(&sidecar_path).with_context(|| format!("read {}", sidecar_path.display()))?,
    )
    .with_context(|| format!("parse sidecar {}", sidecar_path.display()))?;

    let mut agree = 0usize;
    let mut model_clear_engine_blocked = 0usize;
    let mut model_blocked_engine_clear = 0usize;
    let mut disagreements: Vec<String> = Vec::new();
    for &(ox, oy, oz, tx, ty, tz, engine_clear) in &parity.pairs {
        let los = bp.annotate_sight_line(&sidecar, [ox, oy, oz], [tx, ty, tz]);
        if los.is_clear == engine_clear {
            agree += 1;
        } else {
            if los.is_clear {
                model_clear_engine_blocked += 1;
            } else {
                model_blocked_engine_clear += 1;
            }
            if disagreements.len() < 12 {
                disagreements.push(format!(
                    "  obs [{ox:.1},{oy:.1},{oz:.1}] → tgt [{tx:.1},{ty:.1},{tz:.1}]: engine {} vs model {}{}",
                    verdict(engine_clear),
                    verdict(los.is_clear),
                    // Name the terminal hit (wall id, roof, solid, full-cover furniture) on blocks.
                    (!los.is_clear)
                        .then(|| los.hits.last().map(|h| format!(" ({:?} {})", h.kind, h.id)))
                        .flatten()
                        .unwrap_or_default(),
                ));
            }
        }
    }

    let total = parity.pairs.len().max(1);
    println!(
        "parity {}: {agree}/{} agree ({:.1}%) · model-clear/engine-blocked {} · model-blocked/engine-clear {}",
        parity.slug,
        parity.pairs.len(),
        agree as f64 * 100.0 / total as f64,
        model_clear_engine_blocked,
        model_blocked_engine_clear,
    );
    for d in &disagreements {
        println!("{d}");
    }
    Ok(0)
}

fn verdict(clear: bool) -> &'static str {
    if clear { "CLEAR" } else { "BLOCKED" }
}
