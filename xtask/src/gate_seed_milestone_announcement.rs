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
    match proc::which("psql") {
        Ok(_) => {
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
                Run::new("psql")
                    .arg(&url)
                    .arg("-v")
                    .arg("ON_ERROR_STOP=1")
                    .stdin(SQL),
            )
        }
        Err(NotRun::ToolAbsent(_)) => {
            if tbdevent_postgres_running() {
                run_sql(
                    Run::new("podman")
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
    match Run::new("podman")
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
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    fn fixture_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("t872-seed-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::create_dir_all(root.join("apps/website/api")).unwrap();
        fs::create_dir_all(root.join("scripts/mod")).unwrap();
        fs::write(root.join(".ai/tickets/registry.json"), "{}").unwrap();
        // Script path exists only for error-message pin (port does not execute it).
        fs::write(root.join(SCRIPT_REL), "# stub\n").unwrap();
        root
    }

    fn write_env(root: &Path, body: &str) {
        fs::write(root.join("apps/website/api/.env"), body).unwrap();
    }

    #[test]
    fn sql_matches_former_heredoc_len() {
        assert_eq!(SQL.len(), 528); // UTF-8 bytes (3 multi-byte dashes)
        assert!(SQL.ends_with(");\n"));
        assert!(SQL.contains("Milestone #1 — Saturday 22 August 2026"));
    }

    #[test]
    fn no_psql_no_container_exits_1() {
        let _g = LOCK.lock().unwrap();
        let root = fixture_root("no-psql");
        write_env(&root, "DATABASE_URL=postgres://u:p@127.0.0.1:5432/db\n");
        // Isolate: no psql; stub podman with empty ps list.
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("podman"), "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = fs::metadata(bin.join("podman")).unwrap().permissions();
            p.set_mode(0o755);
            fs::set_permissions(bin.join("podman"), p).unwrap();
        }
        let old_path = std::env::var_os("PATH");
        // SAFETY: single-threaded under LOCK; restored below.
        unsafe {
            std::env::set_var("PATH", bin.as_os_str());
            std::env::remove_var("DATABASE_URL");
        }
        let code = run_with_root(&root).unwrap();
        match old_path {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
        let _ = fs::remove_dir_all(&root);
        assert_eq!(code, 1);
    }

    #[test]
    fn missing_env_continues_then_no_psql() {
        let _g = LOCK.lock().unwrap();
        let root = fixture_root("no-env");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("podman"), "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = fs::metadata(bin.join("podman")).unwrap().permissions();
            p.set_mode(0o755);
            fs::set_permissions(bin.join("podman"), p).unwrap();
        }
        let old_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", bin.as_os_str());
            std::env::remove_var("DATABASE_URL");
        }
        let code = run_with_root(&root).unwrap();
        match old_path {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
        let _ = fs::remove_dir_all(&root);
        assert_eq!(code, 1);
    }

    #[test]
    fn bad_database_url_forwards_psql_rc() {
        let _g = LOCK.lock().unwrap();
        let root = fixture_root("bad-url");
        write_env(
            &root,
            "DATABASE_URL=postgres://bad:bad@127.0.0.1:1/nosuch\n",
        );
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            bin.join("psql"),
            "#!/bin/sh\ncat >/dev/null\necho 'psql: error: connection refused' >&2\nexit 2\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = fs::metadata(bin.join("psql")).unwrap().permissions();
            p.set_mode(0o755);
            fs::set_permissions(bin.join("psql"), p).unwrap();
        }
        let old_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", format!("{}:/usr/bin:/bin", bin.display()));
            std::env::remove_var("DATABASE_URL");
        }
        let code = run_with_root(&root).unwrap();
        match old_path {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
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
