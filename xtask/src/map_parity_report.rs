//! Phase B — `cargo xtask map parity-report`: replay the Workbench parity oracle through the
//! Rust 2.5D raycaster and report agreement.
//!
//! The oracle (`EMCP_WB_TbdBlueprint` action `parity`) records engine `TraceMove` verdicts for
//! random observer/target pairs in the building's LOCAL frame — the same frame the blueprint
//! uses — with glass panes excluded (vision passes glass). This command replays every pair
//! through `BuildingBlueprint::evaluate_los` on the extracted blueprint and prints where the
//! 2.5D model and the engine disagree. Report-only: the number is the instrument, not a gate.
//!
//! Usage: `cargo xtask map parity-report --pairs <parity.json> --blueprint <blueprint.json>`

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use map_engine_core::building_blueprint::BuildingBlueprint;

#[derive(serde::Deserialize)]
struct ParityFile {
    slug: String,
    /// `[ox, oy, oz, tx, ty, tz, engineClear]` per row.
    pairs: Vec<(f64, f64, f64, f64, f64, f64, bool)>,
}

pub fn run(args: &[String]) -> Result<u8> {
    let mut pairs_path: Option<PathBuf> = None;
    let mut bp_path: Option<PathBuf> = None;
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
            other => {
                eprintln!(
                    "parity-report: unknown arg {other} (usage: --pairs <json> --blueprint <json>)"
                );
                return Ok(1);
            }
        }
    }
    let (Some(pairs_path), Some(bp_path)) = (pairs_path, bp_path) else {
        eprintln!("parity-report: --pairs and --blueprint are both required");
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

    let mut agree = 0usize;
    let mut model_clear_engine_blocked = 0usize;
    let mut model_blocked_engine_clear = 0usize;
    let mut disagreements: Vec<String> = Vec::new();
    for &(ox, oy, oz, tx, ty, tz, engine_clear) in &parity.pairs {
        let los = bp.evaluate_los([ox, oy, oz], [tx, ty, tz]);
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
                    los.blocked_by_wall_id
                        .as_deref()
                        .map(|w| format!(" ({w})"))
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
