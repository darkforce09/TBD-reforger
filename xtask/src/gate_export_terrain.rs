//! T-869 — port of `scripts/map-assets/export-terrain.sh`
//! → `cargo xtask map export-terrain`.
//!
//! Data-only Map Engine v2 export orchestrator (no raster / tile pyramid):
//! 1. `tbd-tools` `world phase-gate`
//! 2. staged `raw-entities.jsonl` present? else operator instructions + exit 2
//! 3. `world build-objects` then `world build-roads`
//!
//! Exit codes (bash contract): 0 built · 1 bad args / failed build · 2 staged raw missing.
//!
//! Cargo child output is non-reproducible across cold/warm caches — acceptance uses the
//! §Non-reproducible normalised back-to-back recipe from `t853_shell_to_xtask_waves.md`.

use std::env;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Result, bail};
use tbd_gate::proc::Run;
use tbd_gate::verdict::NotRun;

use crate::root::find_repo_root;

/// Entry for `xtask map export-terrain …` (args after the subcommand, bash-shaped).
pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, args)
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(root: &Path, args: &[String]) -> Result<u8> {
    let (terrain, phase) = match parse_args(args) {
        Parse::Usage => {
            eprintln!("usage: export-terrain.sh <terrain> [--phase Pn]   (or TERRAIN env)");
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

    let raw = root
        .join("packages/map-assets")
        .join(&terrain)
        .join("staging/export/raw-entities.jsonl");
    if !raw.is_file() {
        // Unquoted heredoc in bash: $RAW / $TERRAIN / $PHASE expand; \$profile / \$PROFILE_DIR stay.
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
            "       cargo run -q -p tbd-tools --bin world -- copy-export-profile --terrain {terrain} --full \\"
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
        "export-terrain: {terrain} {phase} done — next: cargo run -q -p tbd-tools --bin world -- verify-phase --terrain {terrain} --phase {phase}"
    );
    Ok(0)
}

enum Parse {
    Usage,
    Unknown(String),
    Ok { terrain: String, phase: String },
}

/// Mirror bash: `TERRAIN="${1:-${TERRAIN:-}}"; shift || true;` then `--phase` loop.
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
                    // bash `PHASE="$2"; shift 2` with set -u → unbound $2. Treat as bad args.
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
    // bash: `(cd "$REPO_ROOT" && cargo run -q -p tbd-tools --bin world -- …)`
    let mut args = vec![
        "run".to_string(),
        "-q".to_string(),
        "-p".to_string(),
        "tbd-tools".to_string(),
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
            bail!("cargo run -p tbd-tools --bin world signalled ({signal})")
        }
        Err(NotRun::Timeout { secs, .. }) => {
            bail!("cargo run -p tbd-tools --bin world timed out after {secs}s")
        }
        Err(NotRun::ToolError { tool, stderr, .. }) => {
            bail!("{tool} failed: {stderr}")
        }
        // TargetMissing / Unreadable are file-scan variants; proc::Run does not emit them.
        Err(other) => bail!("cargo run -p tbd-tools --bin world: {other:?}"),
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
mod tests {
    use super::*;
    use std::sync::Mutex;

    // parse_args reads/writes process env (`TERRAIN`) — serialize these tests.
    static TERRAIN_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_terrain_env<R>(f: impl FnOnce() -> R) -> R {
        let _guard = TERRAIN_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = env::var_os("TERRAIN");
        let out = f();
        match prev {
            Some(v) => unsafe { env::set_var("TERRAIN", v) },
            None => unsafe { env::remove_var("TERRAIN") },
        }
        out
    }

    #[test]
    fn parse_usage_when_no_terrain() {
        with_terrain_env(|| {
            unsafe { env::remove_var("TERRAIN") };
            assert!(matches!(parse_args(&[]), Parse::Usage));
        });
    }

    #[test]
    fn parse_unknown_arg() {
        let args = vec![
            "everon".into(), // E2c-allow
            "--bogus".into(),
        ];
        match parse_args(&args) {
            Parse::Unknown(a) => assert_eq!(a, "--bogus"),
            Parse::Usage | Parse::Ok { .. } => panic!("expected Unknown"),
        }
    }

    #[test]
    fn parse_phase_and_default() {
        with_terrain_env(|| {
            unsafe { env::remove_var("TERRAIN") };
            let args_default = vec![
                "everon".into(), // E2c-allow
            ];
            match parse_args(&args_default) {
                Parse::Ok { terrain, phase } => {
                    assert_eq!(terrain, "everon"); // E2c-allow
                    assert_eq!(phase, "P1_buildings");
                }
                _ => panic!("expected Ok"),
            }
            let args_phase = vec![
                "arland".into(), // E2c-allow
                "--phase".into(),
                "P2_trees".into(),
            ];
            match parse_args(&args_phase) {
                Parse::Ok { terrain, phase } => {
                    assert_eq!(terrain, "arland"); // E2c-allow
                    assert_eq!(phase, "P2_trees");
                }
                _ => panic!("expected Ok"),
            }
        });
    }

    #[test]
    fn parse_terrain_from_env_when_no_positional() {
        with_terrain_env(|| {
            unsafe { env::set_var("TERRAIN", "everon") }; // E2c-allow
            // bash: $1=--phase wins over env → terrain="--phase", then "P1_buildings" is unknown.
            match parse_args(&["--phase".into(), "P1_buildings".into()]) {
                Parse::Unknown(a) => assert_eq!(a, "P1_buildings"),
                Parse::Ok { terrain, .. } => panic!("unexpected Ok terrain={terrain}"),
                Parse::Usage => panic!("unexpected Usage"),
            }
            match parse_args(&[]) {
                Parse::Ok { terrain, phase } => {
                    assert_eq!(terrain, "everon"); // E2c-allow
                    assert_eq!(phase, "P1_buildings");
                }
                _ => panic!("expected env terrain"),
            }
        });
    }
}
