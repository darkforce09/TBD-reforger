//! T-872 — port of `scripts/mod/seed-milestone-announcement.sh`
//! → `cargo xtask mod seed-announcement`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `WEB=apps/website/api`. Sources `$WEB/.env` for `DATABASE_URL` (KEY=VALUE parse, not a
//! full shell `source`).
//!
//! Inserts the pinned Milestone #1 website announcement when absent.
//!
//! Fail-opens closed / pinned vs bash:
//! - Missing `$WEB/.env`: bash `set -a && source && set +a` prints the source error and
//!   **continues** (`set -e` does not abort mid `&&`-list) — preserved. Absolute path uses
//!   `$ROOT/scripts/mod/seed-milestone-announcement.sh: line 9: …`.
//! - No `psql` and no running `tbdevent-postgres` container → stderr message, exit **1**.
//! - `psql` present but `DATABASE_URL` unbound (`set -u`) → bash line-12 unbound shape, exit **1**.
//! - `podman` absent / failing / no matching name: treated as "no container" (bash
//!   `2>/dev/null | grep -qx`), never a silent success when psql is also absent.
//! - Child `psql` / `podman exec` nonzero: raw exit code forwarded (bash `set -e`).
//!
//! Test seams (prefer these over PATH stubs — PATH races `gate_crf_leak`'s `/usr/bin/grep`):
//! - `TBD_SEED_MILESTONE_PSQL` — absolute path to a psql binary (checked before `PATH`).
//!   A set-but-missing path forces [`NotRun::ToolAbsent`].
//! - `TBD_SEED_MILESTONE_PODMAN` — same for podman.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tbd_gate::proc::{self, Run};
use tbd_gate::verdict::NotRun;

use crate::root::find_repo_root;

/// Historical script path (error-message pin + inventory identity).
const SCRIPT_REL: &str = "scripts/mod/seed-milestone-announcement.sh";

/// Optional absolute psql path for unit tests (avoids PATH mutation).
const ENV_PSQL: &str = "TBD_SEED_MILESTONE_PSQL";
/// Optional absolute podman path for unit tests (avoids PATH mutation).
const ENV_PODMAN: &str = "TBD_SEED_MILESTONE_PODMAN";

/// Exact `<<'SQL'` body from the former bash (trailing newline included).
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

/// Paths mirroring `scripts/mod/lib/paths.sh`.
struct Paths {
    web: PathBuf,
    script: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            web: root.join("apps/website/api"),
            script: root.join(SCRIPT_REL),
        }
    }
}

/// Entry for `xtask mod seed-announcement`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root)
}

/// Testable entry that does not walk for the repo root (throwaway fixture trees).
pub fn run_with_root(root: &Path) -> Result<u8> {
    let paths = Paths::from_root(root);
    let env_file = paths.web.join(".env");

    // bash: `set -a && source "$WEB/.env" && set +a`
    // Process env first; file overlay overwrites (including empty), matching `source`.
    let mut database_url = std::env::var("DATABASE_URL").ok();

    if env_file.is_file() {
        match parse_dotenv(&env_file) {
            Ok(map) => {
                if let Some(v) = map.get("DATABASE_URL") {
                    database_url = Some(v.clone());
                }
            }
            Err(e) => {
                // Closed: unreadable .env is not a silent empty source.
                eprintln!("could not read {}: {e}", env_file.display());
                return Ok(1);
            }
        }
    } else {
        // Preserved oddity: source error printed, execution continues.
        eprintln!(
            "{}: line 9: {}: No such file or directory",
            paths.script.display(),
            env_file.display()
        );
    }

    // bash: `if command -v psql >/dev/null 2>&1; then …`
    match resolve_tool(ENV_PSQL, "psql") {
        Ok(psql) => {
            let url = match database_url {
                Some(u) => u,
                None => {
                    // bash `set -u`: `psql "$DATABASE_URL"` → unbound variable
                    eprintln!(
                        "{}: line 12: DATABASE_URL: unbound variable",
                        paths.script.display()
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
            // bash lets psql write stdout/stderr directly; re-emit merged text.
            let _ = io::stdout().write_all(out.text.as_bytes());
            if out.code == 0 {
                println!("{SUCCESS}");
                Ok(0)
            } else {
                // bash `set -e`: raw psql/podman exit (often 2 for connection errors).
                Ok(out.code as u8)
            }
        }
        Err(e) => Ok(not_run_exit(&e)),
    }
}

/// bash: `podman ps --format '{{.Names}}' 2>/dev/null | grep -qx tbdevent-postgres`
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
        // ToolAbsent / nonzero / signalled → elif false (bash pipefail+grep miss).
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
mod tests {
    use super::*;
    use crate::test_env;

    fn fixture_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("t872-seed-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::create_dir_all(root.join("apps/website/api")).unwrap();
        fs::create_dir_all(root.join("scripts/mod")).unwrap();
        fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
        // Script path exists only for error-message pin (port does not execute it).
        fs::write(root.join(SCRIPT_REL), "# stub\n").unwrap();
        root
    }

    fn write_env(root: &Path, body: &str) {
        fs::write(root.join("apps/website/api/.env"), body).unwrap();
    }

    fn chmod_755(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = fs::metadata(path).unwrap().permissions();
            p.set_mode(0o755);
            fs::set_permissions(path, p).unwrap();
        }
    }

    fn write_stub(bin: &Path, name: &str, body: &str) -> PathBuf {
        fs::create_dir_all(bin).unwrap();
        let path = bin.join(name);
        fs::write(&path, body).unwrap();
        chmod_755(&path);
        path
    }

    struct OverrideGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl OverrideGuard {
        fn set(key: &'static str, value: &Path) -> Self {
            let previous = std::env::var_os(key);
            // SAFETY: caller holds test_env::lock_env; restored on drop.
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn set_absent(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            // Nonexistent path → resolve_tool ToolAbsent (no PATH wipe).
            let absent =
                std::env::temp_dir().join(format!("t872-absent-{}-{}", key, std::process::id()));
            unsafe { std::env::set_var(key, &absent) };
            Self { key, previous }
        }
    }

    impl Drop for OverrideGuard {
        fn drop(&mut self) {
            unsafe {
                match self.previous.take() {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn sql_matches_former_heredoc_len() {
        assert_eq!(SQL.len(), 528); // UTF-8 bytes (3 multi-byte dashes)
        assert!(SQL.ends_with(");\n"));
        assert!(SQL.contains("Milestone #1 — Saturday 22 August 2026"));
    }

    #[test]
    fn no_psql_no_container_exits_1() {
        let _g = test_env::lock_env();
        let root = fixture_root("no-psql");
        write_env(&root, "DATABASE_URL=postgres://u:p@127.0.0.1:5432/db\n");
        let bin = root.join("bin");
        let podman = write_stub(&bin, "podman", "#!/bin/sh\nexit 0\n");
        let _no_psql = OverrideGuard::set_absent(ENV_PSQL);
        let _podman = OverrideGuard::set(ENV_PODMAN, &podman);
        unsafe { std::env::remove_var("DATABASE_URL") };
        let code = run_with_root(&root).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(code, 1);
    }

    #[test]
    fn missing_env_continues_then_no_psql() {
        let _g = test_env::lock_env();
        let root = fixture_root("no-env");
        let bin = root.join("bin");
        let podman = write_stub(&bin, "podman", "#!/bin/sh\nexit 0\n");
        let _no_psql = OverrideGuard::set_absent(ENV_PSQL);
        let _podman = OverrideGuard::set(ENV_PODMAN, &podman);
        unsafe { std::env::remove_var("DATABASE_URL") };
        let code = run_with_root(&root).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(code, 1);
    }

    #[test]
    fn bad_database_url_forwards_psql_rc() {
        let _g = test_env::lock_env();
        let root = fixture_root("bad-url");
        write_env(
            &root,
            "DATABASE_URL=postgres://bad:bad@127.0.0.1:1/nosuch\n",
        );
        let bin = root.join("bin");
        let psql = write_stub(
            &bin,
            "psql",
            "#!/bin/sh\ncat >/dev/null\necho 'psql: error: connection refused' >&2\nexit 2\n",
        );
        let _psql = OverrideGuard::set(ENV_PSQL, &psql);
        unsafe { std::env::remove_var("DATABASE_URL") };
        let code = run_with_root(&root).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(code, 2);
    }

    #[test]
    fn paths_pin_web_under_apps_website_api() {
        let root = PathBuf::from("/tmp/fake-mono");
        let p = Paths::from_root(&root);
        assert_eq!(p.web, PathBuf::from("/tmp/fake-mono/apps/website/api"));
        assert_eq!(
            p.script,
            PathBuf::from("/tmp/fake-mono/scripts/mod/seed-milestone-announcement.sh")
        );
    }
}
