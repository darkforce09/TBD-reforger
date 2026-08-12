//! T-887 — port of `scripts/deploy/backup-drill.sh` → `cargo xtask deploy db drill`.
//!
//! Restore-into-scratch recoverability proof. CREATE/DROP only allow-listed scratch DBs
//! (`tbd_drill_probe`, etc.). Live `tbd_reforger` is dump SOURCE only — never a restore target
//! (T-381 via [`deploy_db_common::refuse_unsafe_restore_target`]).
//!
//! Calls [`deploy_db_backup`] / [`deploy_db_restore`] in-process (no `emit-bash-fns`).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::deploy_db_backup;
use crate::deploy_db_common::{
    count_db_rows, ct_capture, database_exists, db_user, die, info, refuse_unsafe_restore_target,
    require_container, require_pg_tool, warn,
};
use crate::deploy_db_restore::{self, RestoreArgs};
use crate::root::find_repo_root;

/// Entry for `cargo xtask deploy db drill …` (argv after the `drill` subcommand).
pub fn run(args: &[String]) -> Result<u8> {
    let mut dump: Option<PathBuf> = None;
    let mut source_db = env::var("TBD_BACKUP_DB").unwrap_or_else(|_| "tbd_reforger".into());
    let mut out = env::var("TBD_BACKUP_DIR").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/tbd-backups/website")
    });
    let mut scratch = env::var("TBD_DRILL_DB").unwrap_or_else(|_| "tbd_drill_probe".into());
    let mut fresh = false;
    let mut keep_scratch = false;
    let mut strict_migrations = env::var("TBD_DRILL_STRICT_MIGRATIONS")
        .map(|v| v != "0")
        .unwrap_or(true);

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dump" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--dump needs a value");
                }
                dump = Some(PathBuf::from(v));
                i += 2;
            }
            "--db" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--db needs a value");
                }
                source_db = v.to_string();
                i += 2;
            }
            "--out" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--out needs a value");
                }
                out = v.to_string();
                i += 2;
            }
            "--scratch" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--scratch needs a value");
                }
                scratch = v.to_string();
                i += 2;
            }
            "--fresh" => {
                fresh = true;
                i += 1;
            }
            "--keep-scratch" => {
                keep_scratch = true;
                i += 1;
            }
            "--lax-migrations" => {
                strict_migrations = false;
                i += 1;
            }
            "-h" | "--help" => {
                print_usage(false);
                return Ok(0);
            }
            other => {
                eprintln!("Unknown option: {other}");
                print_usage(true);
                return Ok(2);
            }
        }
    }

    let migdir = env::var("TBD_GATE_MIGRATION_DIR").unwrap_or_else(|_| {
        match find_repo_root() {
            Ok(root) => root
                .join("apps/website/api/migrations")
                .display()
                .to_string(),
            Err(_) => {
                // bash expanded $TBD_MONO_ROOT (set by deleted db-common.sh). Fail closed.
                die(
                    "TBD_GATE_MIGRATION_DIR unset and repo root not found — cannot locate migrations.",
                );
            }
        }
    });

    // Same T-381 guard as manual restore — drill must never aim at live.
    refuse_unsafe_restore_target(&scratch, None)?;

    if which("sha384sum").is_none() {
        die(
            "sha384sum is not on PATH — the migration checksum audit cannot run, and a drill that skips it is not a drill.",
        );
    }

    require_container()?;
    let _ = require_pg_tool("pg_restore")?;
    let _ = require_pg_tool("psql")?;

    let mut failed = false;
    let mut note_fail = |msg: &str| {
        eprintln!("DRILL FAIL: {msg}");
        failed = true;
    };

    println!("═══ backup restore drill ═══");

    if fresh {
        info(&format!("taking a fresh backup of '{source_db}' first"));
        let rc = deploy_db_backup::run(&[
            "--db".into(),
            source_db.clone(),
            "--out".into(),
            out.clone(),
        ])?;
        if rc != 0 {
            die("the fresh backup failed; nothing to drill.");
        }
    }

    let dump = match dump {
        Some(p) => p,
        None => {
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Ok(entries) = fs::read_dir(&out) {
                for ent in entries.flatten() {
                    let path = ent.path();
                    if !path.is_file() {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    // Match bash: "$OUT/${SOURCE_DB}-"*.dump
                    if name.starts_with(&format!("{source_db}-")) && name.ends_with(".dump") {
                        candidates.push(path);
                    }
                }
            }
            // LC_ALL=C sort -r
            candidates.sort_by(|a, b| {
                let sa = a.to_string_lossy();
                let sb = b.to_string_lossy();
                sb.cmp(&sa)
            });
            if candidates.is_empty() {
                die(&format!(
                    "no backups matching '{out}/{source_db}-*.dump'.
       THERE IS NOTHING TO RECOVER FROM. This is the loudest possible result and it is correct:
       an empty backup directory is the failure the drill exists to surface."
                ));
            }
            info(&format!(
                "drilling newest of {} backup(s)",
                candidates.len()
            ));
            candidates.into_iter().next().unwrap()
        }
    };

    info(&format!("dump       {}", dump.display()));
    let age_h = dump_age_hours(&dump);
    info(&format!("age        {age_h} hour(s) old"));

    let _guard = ScratchGuard {
        scratch: scratch.clone(),
        keep: keep_scratch,
    };

    // Drop scratch before restore (stdout/stderr discarded — bash `>/dev/null 2>&1`).
    let _ = drop_scratch_db(&scratch);

    info(&format!("restoring into scratch database '{scratch}'"));
    // --expect-db is the SOURCE database, not scratch (T-588).
    let restore_rc = deploy_db_restore::run(RestoreArgs {
        db: Some(scratch.clone()),
        url: None,
        create: true,
        jobs: default_restore_jobs(),
        min_rows: default_restore_min_rows(),
        expect_db: Some(source_db.clone()),
        confirm: None,
        dump: dump.clone(),
    })?;
    if restore_rc != 0 {
        eprintln!("DRILL FAIL: the backup could NOT be restored. It is not a usable backup.");
        return Ok(1);
    }

    let rows = count_db_rows(&scratch)?;
    let tables = psql_scalar(
        &scratch,
        "SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE';",
    );
    let enums = psql_scalar(
        &scratch,
        "SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE t.typtype='e' AND n.nspname='public';",
    );
    let idx = psql_scalar(
        &scratch,
        "SELECT count(*) FROM pg_indexes WHERE schemaname='public';",
    );
    info(&format!(
        "restored   {} tables · {} enums · {} indexes · {} rows",
        if tables.is_empty() { "?" } else { &tables },
        if enums.is_empty() { "?" } else { &enums },
        if idx.is_empty() { "?" } else { &idx },
        if rows.is_empty() { "?" } else { &rows },
    ));
    let tables_n: i64 = tables.parse().unwrap_or(0);
    if tables_n <= 0 {
        note_fail("the restored database has no tables.");
    }

    if database_exists(&source_db)? {
        let src_tables = psql_scalar(
            &source_db,
            "SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE';",
        );
        if !src_tables.is_empty() && src_tables != tables {
            note_fail(&format!(
                "table count differs from the source: '{source_db}' has {src_tables}, the restore has {tables}."
            ));
        } else if !src_tables.is_empty() {
            info(&format!(
                "parity     table count matches the live source ({src_tables})"
            ));
        }
    }

    info(&format!("boot check migration state vs {migdir}"));
    let migdir_path = PathBuf::from(&migdir);
    if !migdir_path.is_dir() {
        note_fail(&format!(
            "migration directory '{migdir}' does not exist — the boot-readiness audit could not run (fail closed)."
        ));
    } else {
        let mut migfiles: Vec<PathBuf> = fs::read_dir(&migdir_path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sql"))
            .collect();
        migfiles.sort();
        if migfiles.is_empty() {
            note_fail(&format!(
                "no .sql migrations under '{migdir}' — refusing to report a boot-ready restore from an audit with nothing to compare."
            ));
        } else {
            let has_tbl = psql_scalar(
                &scratch,
                "SELECT to_regclass('public._sqlx_migrations') IS NOT NULL;",
            );
            if has_tbl != "t" {
                let msg = "the restore has NO _sqlx_migrations table. On boot, sqlx would try to apply 0001 over the\n            tables this restore just recreated, CREATE TABLE would fail, and the API would not start.";
                if strict_migrations {
                    note_fail(msg);
                } else {
                    warn(msg);
                }
            } else {
                let applied = psql_query(
                    &scratch,
                    "SELECT version || '|' || (CASE WHEN success THEN 'ok' ELSE 'bad' END) || '|' || encode(checksum,'hex') FROM _sqlx_migrations ORDER BY version;",
                );
                let mut drift = 0u32;
                let mut okn = 0u32;
                let mut badn = 0u32;
                let mut unknown = 0u32;
                for line in applied.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let mut parts = line.splitn(3, '|');
                    let ver = parts.next().unwrap_or("");
                    let state = parts.next().unwrap_or("");
                    let sum = parts.next().unwrap_or("");
                    if ver.is_empty() {
                        continue;
                    }
                    let f = migfiles.iter().find(|cand| mig_ver(cand) == ver);
                    let Some(f) = f else {
                        unknown += 1;
                        note_fail(&format!(
                            "restore records migration {ver}, which has no file in {migdir}."
                        ));
                        continue;
                    };
                    if state != "ok" {
                        badn += 1;
                        note_fail(&format!(
                            "migration {ver} is recorded as NOT successful in the restore."
                        ));
                    }
                    let disk = sha384_hex(f);
                    if disk != sum {
                        drift += 1;
                        note_fail(&format!(
                            "migration {ver} ({}) checksum drift — the API would refuse to boot with sqlx VersionMismatch.\n\
            recorded: {sum}\n\
            on disk:  {disk}",
                            f.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                        ));
                    } else {
                        okn += 1;
                    }
                }

                let allv_raw = psql_scalar(
                    &scratch,
                    "SELECT string_agg(version::text,' ') FROM _sqlx_migrations;",
                );
                let allv = format!(" {allv_raw} ");
                let mut pending = 0u32;
                for f in &migfiles {
                    let v = mig_ver(f);
                    let needle = format!(" {v} ");
                    if !allv.contains(&needle) {
                        pending += 1;
                    }
                }
                info(&format!(
                    "boot check {okn} migration(s) match on disk · {pending} pending · {drift} drifted · {badn} failed · {unknown} unknown"
                ));
                if pending > 0 {
                    info(&format!(
                        "           ({pending} newer migration(s) would be applied on boot — expected after a deploy)"
                    ));
                }
            }
        }
    }

    println!();
    if !failed {
        println!(
            "DRILL PASS — {} restored into '{scratch}' with {} row(s) across {} table(s), and is boot-ready.",
            dump.display(),
            rows,
            tables
        );
        Ok(0)
    } else {
        eprintln!("DRILL FAIL — the backups are NOT proven recoverable. See the failures above.");
        Ok(1)
    }
}

fn print_usage(to_stderr: bool) {
    let text = "\
Usage: cargo xtask deploy db drill [--dump FILE] [--fresh] [--db NAME] [--scratch NAME] [--keep-scratch]

  Restore a backup into a scratch database and prove it is recoverable AND bootable.

  --dump FILE      drill this dump          (default: newest in the backup dir)
  --fresh          take a new backup first, then drill that
  --db NAME        source database name     (default $TBD_BACKUP_DB, or tbd_reforger)
  --out DIR        backup directory         (default $TBD_BACKUP_DIR)
  --scratch NAME   scratch restore target   (default tbd_drill_probe; must be allow-listed)
  --keep-scratch   do not drop the scratch database at the end
  --lax-migrations warn instead of fail when the restore is not boot-ready
  -h, --help       show this help

Exit: 0 the backup is recoverable · 1 it is NOT · 2 usage · 3 missing library
";
    if to_stderr {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
}

fn default_restore_jobs() -> u32 {
    env::var("TBD_RESTORE_JOBS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(1)
}

fn default_restore_min_rows() -> u64 {
    env::var("TBD_RESTORE_MIN_ROWS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn which(prog: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let p = dir.join(prog);
            if p.is_file() { Some(p) } else { None }
        })
    })
}

fn dump_age_hours(dump: &Path) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mtime = fs::metadata(dump)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(now);
    now.saturating_sub(mtime) / 3600
}

/// sed 's/^0*\([0-9][0-9]*\)_.*/\1/' on the basename.
fn mig_ver(path: &Path) -> String {
    let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let bytes = base.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'0' {
        i += 1;
    }
    let start = i;
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        // All leading zeros consumed and no digit left — e.g. "000_" → last zero is the digit.
        // sed: 0* then [0-9][0-9]* — backtrack so one zero remains as the digit match.
        if start > 0 && start < bytes.len() && bytes[start] == b'_' {
            return "0".to_string();
        }
        return base.to_string();
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'_' {
        base[start..i].to_string()
    } else {
        base.to_string()
    }
}

fn sha384_hex(path: &Path) -> String {
    let output = Command::new("sha384sum").arg(path).output();
    match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.split_whitespace().next().unwrap_or("").to_string()
        }
        _ => String::new(),
    }
}

fn psql_scalar(db: &str, sql: &str) -> String {
    let user = db_user();
    match ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.to_string(),
        ],
    ) {
        Ok((_rc, stdout, _stderr)) => stdout.trim().to_string(),
        Err(_) => String::new(),
    }
}

fn psql_query(db: &str, sql: &str) -> String {
    // Same as scalar but preserve newlines (ORDER BY version rows).
    let user = db_user();
    match ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.to_string(),
        ],
    ) {
        Ok((_rc, stdout, _stderr)) => stdout,
        Err(_) => String::new(),
    }
}

fn drop_scratch_db(scratch: &str) -> Result<()> {
    let user = db_user();
    let sql = format!("DROP DATABASE IF EXISTS \"{scratch}\" WITH (FORCE);");
    let _ = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            "postgres".into(),
            "-qc".into(),
            sql,
        ],
    )?;
    Ok(())
}

struct ScratchGuard {
    scratch: String,
    keep: bool,
}

impl Drop for ScratchGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = drop_scratch_db(&self.scratch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mig_ver;
    use std::path::Path;

    #[test]
    fn mig_ver_strips_leading_zeros() {
        assert_eq!(mig_ver(Path::new("0001_init.sql")), "1");
        assert_eq!(mig_ver(Path::new("0021_later.sql")), "21");
        assert_eq!(mig_ver(Path::new("10_ten.sql")), "10");
    }
}
