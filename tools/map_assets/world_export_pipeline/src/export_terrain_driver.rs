//! `cargo xtask map export-terrain`: the data-only export driver of a terrain (no raster and no
//! tile pyramid).
//!
//! **Role:** runs the `world` binary's export stages for one terrain and phase, in order:
//! 1. `world phase-gate`;
//! 2. the staged `raw-entities.jsonl` must exist, else the operator steps are printed and the run
//!    exits 2;
//! 3. `world build-objects`, then `world build-roads`.
//!
//! **Position:** xtask's `map` dispatch calls [`run`](crate::export_terrain_driver::run); each
//! stage is `cargo run -q -p developer_tools --bin world -- …` in the checkout root through
//! `process_runner`, with the child's output printed when it ends; the export validation's E2b
//! check runs the command and expects exit 2 for a terrain with no staged export.
//! **Signals & state:** none held; reads `TERRAIN` from the environment when no terrain argument
//! is given.
//! **Invariants:** exit codes are 0 built, 1 a bad argument or a failed build, 2 the staged raw
//! export is missing, 127 cargo is not installed, otherwise the failing stage's own code; the phase
//! gate runs before anything is built, and nothing is built over a missing export. Cargo's child
//! output differs between cold and warm build caches, so two runs compare only after that output
//! is normalised away.

use std::env;
use std::io::{self, Write};
use std::path::Path;

use process_runner::Run;
use repository_layout::{find_repository_root, map_scratch_dir};
use verification_core::verdict::NotRun;

use crate::error::{Result, refuse};

/// Entry for `cargo xtask map export-terrain …`: `args` are the arguments after the subcommand.
pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repository_root()?;
    run_with_root(&root, args)
}

/// [`run`] against an explicit checkout root.
pub fn run_with_root(root: &Path, args: &[String]) -> Result<u8> {
    let (terrain, phase) = match parse_args(args) {
        Parse::Usage => {
            eprintln!(
                "usage: cargo xtask map export-terrain <terrain> [--phase Pn]   (or TERRAIN env)"
            );
            return Ok(1);
        }
        Parse::Unknown(arg) => {
            eprintln!("export-terrain: unknown arg {arg}");
            return Ok(1);
        }
        Parse::Ok { terrain, phase } => (terrain, phase),
    };

    // Phase gate: requested phase must not exceed registry importPhaseMax.
    let rc = world_cargo(
        root,
        &["phase-gate", "--terrain", &terrain, "--phase", &phase],
    )?;
    if rc != 0 {
        return Ok(rc);
    }

    let raw = map_scratch_dir(root, &terrain).join("export/raw-entities.jsonl");
    if !raw.is_file() {
        // The terrain, phase and staged path are filled in; `$profile` and `$PROFILE_DIR` are
        // printed literally for the operator's shell.
        eprintln!("export-terrain: staged raw export missing for '{terrain}':");
        eprintln!("  {}", raw.display());
        eprintln!();
        eprintln!("Operator step (one Workbench run per terrain per export):");
        eprintln!(
            "  1. Workbench: open the terrain world with all layers loaded (wb_state should report ~1M+ entities)"
        );
        eprintln!("  2. Run the full-world export — either:");
        eprintln!(
            "       MCP:    MCP_CALL_TIMEOUT=3600 cargo run -q -p xtask -- mcp call wb_execute_action \\"
        );
        eprintln!(
            "                 '{{\"menuPath\":\"Plugins,TBD,Export TBD World Objects (full)\"}}'"
        );
        eprintln!("       Manual: Workbench > Plugins > TBD > \"Export TBD World Objects (full)\"");
        eprintln!(
            "     The plugin iterates 512 m cell passes and writes $profile:TBD_WorldExport_full.jsonl,"
        );
        eprintln!(
            "     then TBD_WorldExport_full_meta.json (meta = completion sentinel — written last)."
        );
        eprintln!("  3. Stage it:");
        eprintln!(
            "       cargo run -q -p developer_tools --bin world -- copy-export-profile --terrain {terrain} --full \\"
        );
        eprintln!("         --profile \"$PROFILE_DIR\"");
        eprintln!("  4. Re-run: cargo xtask map export-terrain {terrain} --phase {phase}");
        return Ok(2);
    }

    println!("export-terrain: {terrain} {phase} — building catalog artifacts");
    let rc = world_cargo(
        root,
        &[
            "build-objects",
            "--terrain",
            &terrain,
            "--phase",
            &phase,
            "--patch-manifest",
            "--ops-log",
        ],
    )?;
    if rc != 0 {
        return Ok(rc);
    }
    let rc = world_cargo(root, &["build-roads", "--terrain", &terrain, "--ops-log"])?;
    if rc != 0 {
        return Ok(rc);
    }
    println!(
        "export-terrain: {terrain} {phase} done — next: cargo run -q -p developer_tools --bin world -- verify-phase --terrain {terrain} --phase {phase}"
    );
    Ok(0)
}

enum Parse {
    Usage,
    Unknown(String),
    Ok { terrain: String, phase: String },
}

/// The first argument is the terrain (else `TERRAIN`, else usage), even when it starts with
/// `--`; the rest are `--phase <phase>` pairs, the last one winning, and anything else is refused.
fn parse_args(args: &[String]) -> Parse {
    let mut idx = 0;
    let terrain = if let Some(first) = args.first() {
        idx = 1;
        first.clone()
    } else {
        env::var("TERRAIN").unwrap_or_default()
    };
    if terrain.is_empty() {
        return Parse::Usage;
    }

    let mut phase = "P1_buildings".to_string();
    while idx < args.len() {
        match args[idx].as_str() {
            "--phase" => {
                let Some(val) = args.get(idx + 1) else {
                    // `--phase` without a value is a bad argument.
                    return Parse::Unknown("--phase".into());
                };
                phase = val.clone();
                idx += 2;
            }
            other => return Parse::Unknown(other.to_string()),
        }
    }
    Parse::Ok { terrain, phase }
}

fn world_cargo(root: &Path, world_args: &[&str]) -> Result<u8> {
    // `cargo run -q -p developer_tools --bin world -- …` in the checkout root.
    let mut args = vec![
        "run".to_string(),
        "-q".to_string(),
        "-p".to_string(),
        "developer_tools".to_string(),
        "--bin".to_string(),
        "world".to_string(),
        "--".to_string(),
    ];
    args.extend(world_args.iter().map(|s| (*s).to_string()));

    match Run::new("cargo").args(args).cwd(root).output() {
        Ok(o) => {
            let mut out = io::stdout().lock();
            out.write_all(o.stdout.as_bytes())?;
            out.flush()?;
            let mut err = io::stderr().lock();
            err.write_all(o.stderr.as_bytes())?;
            err.flush()?;
            Ok(exit_u8(o.code))
        }
        Err(NotRun::ToolAbsent(_)) => {
            // Shell "command not found" → 127.
            Ok(127)
        }
        Err(NotRun::Signalled { signal, .. }) => {
            refuse!("cargo run -p developer_tools --bin world signalled ({signal})")
        }
        Err(NotRun::Timeout { secs, .. }) => {
            refuse!("cargo run -p developer_tools --bin world timed out after {secs}s")
        }
        Err(NotRun::ToolError { tool, stderr, .. }) => {
            refuse!("{tool} failed: {stderr}")
        }
        // TargetMissing / Unreadable are file-scan variants; `process_runner::Run` does not emit them.
        Err(other) => refuse!("cargo run -p developer_tools --bin world: {other:?}"),
    }
}

fn exit_u8(code: i32) -> u8 {
    if (0..=255).contains(&code) {
        code as u8
    } else {
        1
    }
}

#[cfg(test)]
#[path = "tests/export_terrain_driver/tests.rs"]
mod tests;
