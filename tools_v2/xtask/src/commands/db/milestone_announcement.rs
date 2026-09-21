//! `cargo xtask mod seed-announcement` — insert the pinned first-milestone website announcement
//! when it is not already there.
//!
//! `DATABASE_URL` comes from the process environment, overlaid by `apps/website/api_v2/.env`,
//! which is parsed as `KEY=VALUE` and never executed.
//!
//! What it refuses and what it tolerates:
//! - An absent `.env` is reported and the command continues: the value may be in the
//!   environment already.
//! - An unreadable `.env` exits 1 rather than proceeding with an empty overlay.
//! - No `psql` and no running `tbdevent-postgres` container exits 1. A `podman` that is absent,
//!   fails, or names no matching container reads as "no container" — never as a success when
//!   `psql` is also missing.
//! - A `psql` with no `DATABASE_URL` to give it exits 1.
//! - A non-zero `psql` or `podman exec` forwards its own exit code.
//!
//! Test seams, preferred over PATH stubs because PATH is process-wide and other checks resolve
//! their own tools through it:
//! - `TBD_SEED_MILESTONE_PSQL` — absolute path to a psql binary, checked before `PATH`. A path
//!   that is set but missing forces [`NotRun::ToolAbsent`].
//! - `TBD_SEED_MILESTONE_PODMAN` — the same for podman.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use crate::core::repository_root::find_repo_root;

/// Optional absolute psql path for unit tests (avoids PATH mutation).
const ENV_PSQL: &str = "TBD_SEED_MILESTONE_PSQL";
/// Optional absolute podman path for unit tests (avoids PATH mutation).
const ENV_PODMAN: &str = "TBD_SEED_MILESTONE_PODMAN";

/// The statement, written so a second run inserts nothing.
const SQL: &str = r#"INSERT INTO announcements (title, body, pinned, published, published_at)
SELECT
  'Milestone #1 — Saturday 22 August 2026',
  E'Our first **manual TBD PvP event** target is **Saturday 21 August 2026** (internal test, 20–40 players).

Mission loads from the backend; ORBAT slots enforce roles; VOIP is optional.

Sign up under **Events**. Mission Wizard arrives in Phase 2 — Milestone #1 uses hand-written JSON.',
  TRUE,
  TRUE,
  NOW()
WHERE NOT EXISTS (
  SELECT 1 FROM announcements WHERE title LIKE 'Milestone #1%'
);
"#;

const SUCCESS: &str = "Website announcement seeded (if not already present).";
const NO_PSQL: &str = "No psql and tbdevent-postgres container not running.";

/// The checkout location this command reads.
struct Paths {
    web: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            web: root.join("apps/website/api_v2"),
        }
    }
}

/// How this command names itself in its own error messages.
const COMMAND: &str = "cargo xtask mod seed-announcement";

/// Entry for `xtask mod seed-announcement`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root)
}

/// Testable entry that does not walk for the repo root (throwaway fixture trees).
pub fn run_with_root(root: &Path) -> Result<u8> {
    let paths = Paths::from_root(root);
    let env_file = paths.web.join(".env");

    // The process environment first; the file's keys then win, empty included.
    let mut database_url = std::env::var("DATABASE_URL").ok();

    if env_file.is_file() {
        match parse_dotenv(&env_file) {
            Ok(map) => {
                if let Some(v) = map.get("DATABASE_URL") {
                    database_url = Some(v.clone());
                }
            }
            Err(e) => {
                // An unreadable `.env` is not an empty one: it stops the command.
                eprintln!("could not read {}: {e}", env_file.display());
                return Ok(1);
            }
        }
    } else {
        // Reported, not fatal: `DATABASE_URL` may already be in the environment.
        eprintln!(
            "{COMMAND}: {}: No such file or directory",
            env_file.display()
        );
    }

    match resolve_tool(ENV_PSQL, "psql") {
        Ok(psql) => {
            let url = match database_url {
                Some(u) => u,
                None => {
                    eprintln!(
                        "{COMMAND}: DATABASE_URL is set in neither the environment nor {}",
                        env_file.display()
                    );
                    return Ok(1);
                }
            };
            run_sql(
                Run::new(psql)
                    .arg(&url)
                    .arg("-v")
                    .arg("ON_ERROR_STOP=1")
                    .stdin(SQL),
            )
        }
        Err(NotRun::ToolAbsent(_)) => {
            if tbdevent_postgres_running() {
                let podman = match resolve_tool(ENV_PODMAN, "podman") {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("{NO_PSQL}");
                        return Ok(1);
                    }
                };
                run_sql(
                    Run::new(podman)
                        .arg("exec")
                        .arg("-i")
                        .arg("tbdevent-postgres")
                        .arg("psql")
                        .arg("-U")
                        .arg("tbdevent")
                        .arg("-d")
                        .arg("tbdevent")
                        .arg("-v")
                        .arg("ON_ERROR_STOP=1")
                        .stdin(SQL),
                )
            } else {
                eprintln!("{NO_PSQL}");
                Ok(1)
            }
        }
        Err(e) => Ok(not_run_exit(&e)),
    }
}

/// Resolve a tool: optional absolute override env, else `PATH` via [`proc::which`].
///
/// A set-but-missing override path is [`NotRun::ToolAbsent`] (test seam for "no psql" without
/// wiping `PATH`). Empty / unset override falls through to `which`.
fn resolve_tool(env_key: &str, name: &str) -> Result<PathBuf, NotRun> {
    if let Ok(override_path) = std::env::var(env_key) {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.is_file() {
                return Ok(p);
            }
            return Err(NotRun::ToolAbsent(name.to_string()));
        }
    }
    proc::which(name)
}

fn run_sql(run: Run) -> Result<u8> {
    match run.merged_output() {
        Ok(out) => {
            // The child's own output is what an operator needs; re-emit it merged.
            let _ = io::stdout().write_all(out.text.as_bytes());
            if out.code == 0 {
                println!("{SUCCESS}");
                Ok(0)
            } else {
                // Forward the child's own exit code (often 2 for a connection error).
                Ok(out.code as u8)
            }
        }
        Err(e) => Ok(not_run_exit(&e)),
    }
}

/// Whether the development database container is running.
fn tbdevent_postgres_running() -> bool {
    let Ok(podman) = resolve_tool(ENV_PODMAN, "podman") else {
        return false;
    };
    match Run::new(podman)
        .arg("ps")
        .arg("--format")
        .arg("{{.Names}}")
        .merged_output()
    {
        Ok(out) if out.code == 0 => out.text.lines().any(|l| l == "tbdevent-postgres"),
        // An absent, failing or signalled podman reads as "no container".
        _ => false,
    }
}

/// KEY=VALUE parser for `$WEB/.env` (not a full shell `source`).
fn parse_dotenv(path: &Path) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(k.trim().to_string(), v);
        }
    }
    Ok(map)
}

fn not_run_exit(e: &NotRun) -> u8 {
    match e {
        NotRun::ToolAbsent(tool) => {
            eprintln!("{tool}: command not found");
            127
        }
        other => {
            eprintln!("{other:?}");
            1
        }
    }
}

#[cfg(test)]
#[path = "tests/milestone_announcement/tests.rs"]
mod tests;
