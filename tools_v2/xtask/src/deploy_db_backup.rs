//! T-885 — port of `scripts/deploy/backup-db.sh` → `cargo xtask deploy db backup`.
//!
//! Verified `pg_dump -Fc` + count-based retention + `--verify-only`. Calls
//! [`crate::deploy_db_common`] helpers directly (no `emit-bash-fns` on this path).
//!
//! Closed fail-opens: none introduced here — dump stays on `.part` until
//! [`deploy_db_common::verify_dump`] holds; promotion is the only success path.
//! Preserved oddity: `chmod 600` errors are ignored (`2>/dev/null || true` in bash) —
//! that does not claim the backup succeeded when verification failed.

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use crate::deploy_db_common::{
    self, count_db_rows, ct_i_to_files, database_exists, db_container, db_user, die, info,
    require_container, require_pg_tool, verify_dump, warn,
};

/// Entry for `cargo xtask deploy db backup …` (argv after the `backup` subcommand).
pub fn run(args: &[String]) -> Result<u8> {
    let mut db = env::var("TBD_BACKUP_DB").unwrap_or_else(|_| "tbd_reforger".into());
    let mut out = env::var("TBD_BACKUP_DIR").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/tbd-backups/website")
    });
    let mut keep = env::var("TBD_BACKUP_KEEP").unwrap_or_else(|_| "14".into());
    let mut min_rows = env::var("TBD_BACKUP_MIN_ROWS").unwrap_or_else(|_| "1".into());
    let mut verify_only: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    // bash: ${2:?--db needs a value} — exits non-zero with that message
                    die("--db needs a value");
                }
                db = v.to_string();
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
            "--keep" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--keep needs a value");
                }
                keep = v.to_string();
                i += 2;
            }
            "--min-rows" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--min-rows needs a value");
                }
                min_rows = v.to_string();
                i += 2;
            }
            "--verify-only" => {
                let v = args.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                if v.is_empty() {
                    die("--verify-only needs a value");
                }
                verify_only = Some(PathBuf::from(v));
                i += 2;
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

    let keep_n = parse_nonneg(&keep, "--keep");
    let min_rows_n = parse_nonneg(&min_rows, "--min-rows");
    if keep_n < 1 {
        die("--keep must be >= 1 — a retention policy that keeps zero backups is not a policy.");
    }

    if let Some(file) = verify_only {
        return run_verify_only(&file, min_rows_n, &db);
    }

    run_backup(&db, &out, keep_n, min_rows_n)
}

fn parse_nonneg(raw: &str, flag: &str) -> u64 {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        die(&format!(
            "{flag} must be a non-negative integer (got '{raw}')"
        ));
    }
    raw.parse::<u64>().unwrap_or_else(|_| {
        die(&format!(
            "{flag} must be a non-negative integer (got '{raw}')"
        ))
    })
}

fn print_usage(to_stderr: bool) {
    let text = "\
Usage: cargo xtask deploy db backup [--db NAME] [--out DIR] [--keep N] [--min-rows N] [--verify-only FILE]

  Dump a PostgreSQL database through the compose container in custom format (-Fc),
  VERIFY the resulting file by reading it back, and prune old backups BY COUNT.

  --db NAME           database to dump          (default $TBD_BACKUP_DB, or tbd_reforger)
  --out DIR           backup directory          (default $TBD_BACKUP_DIR, or ~/tbd-backups/website)
  --keep N            retain the newest N dumps (default $TBD_BACKUP_KEEP, or 14). N>=1.
  --min-rows N        fail if the dump contains fewer than N data rows (default 1)
  --verify-only FILE  verify an existing dump and exit; take no new backup
  -h, --help          show this help

Environment:
  TBD_DB_CONTAINER      compose container name   (default tbd_reforger_db)
  TBD_DB_USER           postgres role            (default tbd)
  TBD_CONTAINER_RUNTIME override runtime, e.g. \"distrobox-host-exec podman\"

Exit: 0 verified backup written · 1 failure (nothing promoted) · 2 usage · 3 missing library
";
    if to_stderr {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
}

fn run_verify_only(file: &Path, min_rows: u64, db: &str) -> Result<u8> {
    require_container()?;
    let _ = require_pg_tool("pg_restore")?;
    info(&format!("verifying {}", file.display()));
    match verify_dump(file, min_rows, Some(db)) {
        Ok(rows) => {
            println!(
                "OK: {} verified — {rows} data row(s), TOC and full archive body read back.",
                file.display()
            );
            Ok(0)
        }
        Err(deploy_db_common::VerifyFail) => {
            eprintln!("FAIL: {} did NOT verify.", file.display());
            Ok(1)
        }
    }
}

fn run_backup(db: &str, out: &str, keep: u64, min_rows: u64) -> Result<u8> {
    if db.is_empty() || !db.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        die(&format!("--db '{db}' is not a plain ASCII database name."));
    }
    if out.is_empty() {
        die("--out is empty.");
    }
    match out {
        "/" | "/root" | "/home" | "/usr" | "/etc" | "/var" => {
            die(&format!("refusing to use '{out}' as a backup directory."));
        }
        _ => {}
    }

    require_container()?;
    let _ = require_pg_tool("pg_dump")?;
    let _ = require_pg_tool("pg_restore")?;

    if !database_exists(db)? {
        die(&format!(
            "database '{db}' does not exist in container '{}'.\n\
       Refusing: pg_dump would fail and leave a zero-byte file that looks like a backup.",
            db_container()
        ));
    }

    fs::create_dir_all(out)
        .unwrap_or_else(|_| die(&format!("cannot create backup directory '{out}'")));
    // bash: `[ -w "$OUT" ]` — probe create+unlink matches the intent on Unix.
    {
        let probe = PathBuf::from(out).join(".tbd-backup-write-probe");
        match fs::File::create(&probe) {
            Ok(_) => {
                let _ = fs::remove_file(&probe);
            }
            Err(_) => die(&format!("backup directory '{out}' is not writable")),
        }
    }

    let stamp = utc_stamp();
    let final_path = PathBuf::from(out).join(format!("{db}-{stamp}.dump"));
    let part_path = PathBuf::from(format!("{}.part", final_path.display()));
    let _guard = PartGuard {
        path: part_path.clone(),
    };

    // bash oddity after T-884: TBD_RUNTIME array is unset (emit-bash-fns stubs resolve), so
    // `${TBD_RUNTIME[*]}` prints empty. Match that rather than inventing a display change.
    let runtime_display = env::var("TBD_RUNTIME").unwrap_or_default();
    info(&format!(
        "database   {db} (container {}, runtime {runtime_display})",
        db_container()
    ));
    info(&format!("target     {}", final_path.display()));

    let mut src_rows: Option<u64> = None;
    match count_db_rows(db) {
        Ok(raw) => {
            let trimmed: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            if !trimmed.is_empty() && trimmed.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(n) = trimmed.parse::<u64>() {
                    src_rows = Some(n);
                    info(&format!("source     {n} live row(s) across user tables"));
                }
            }
        }
        Err(_) => {
            // bash: set -u -o pipefail without -e — a failed count leaves SRC_ROWS empty.
        }
    }

    let dump_err = env::temp_dir().join(format!("tbd-backup-dump-err-{}.txt", std::process::id()));
    info("dumping…");
    let user = db_user();
    let dump_rc = ct_i_to_files(
        &[
            "pg_dump".into(),
            "-U".into(),
            user,
            "-Fc".into(),
            "-d".into(),
            db.to_string(),
        ],
        &part_path,
        &dump_err,
    )?;
    if dump_rc != 0 {
        eprintln!("FAIL: pg_dump exited {dump_rc} — no backup written.");
        if let Ok(f) = fs::File::open(&dump_err) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                eprintln!("      {line}");
            }
        }
        let _ = fs::remove_file(&dump_err);
        return Ok(1);
    }
    if let Ok(meta) = fs::metadata(&dump_err)
        && meta.len() > 0
    {
        warn("pg_dump wrote to stderr:");
        if let Ok(f) = fs::File::open(&dump_err) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                eprintln!("      {line}");
            }
        }
    }
    let _ = fs::remove_file(&dump_err);

    info("verifying the file just written…");
    let rows = match verify_dump(&part_path, min_rows, Some(db)) {
        Ok(r) => r,
        Err(deploy_db_common::VerifyFail) => {
            eprintln!(
                "FAIL: the dump did NOT verify — refusing to promote it to {}.",
                final_path.display()
            );
            eprintln!(
                "      The partial file has been removed. THERE IS NO NEW BACKUP; the previous ones are untouched."
            );
            return Ok(1);
        }
    };

    if let Some(src) = src_rows
        && src > 0
        && rows < src / 2
    {
        warn(&format!(
            "dump holds {rows} row(s) but the database reports {src} — less than half. Investigate."
        ));
    }

    fs::rename(&part_path, &final_path).unwrap_or_else(|_| {
        die(&format!(
            "verified dump could not be moved into place at {}",
            final_path.display()
        ))
    });
    // disarm guard — file no longer at .part
    std::mem::forget(_guard);

    // bash: chmod 600 "$FINAL" 2>/dev/null || true
    let _ = fs::set_permissions(&final_path, fs::Permissions::from_mode(0o600));

    let bytes = fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
    info(&format!(
        "VERIFIED  {} ({bytes} bytes, {rows} data rows)",
        final_path.display()
    ));

    prune_retention(out, db, &final_path, keep)?;
    info("done");
    Ok(0)
}

fn prune_retention(out: &str, db: &str, final_path: &Path, keep: u64) -> Result<()> {
    // Collect ${OUT}/${DB}-*.dump regular files; sort LC_ALL=C reverse (filename = chrono).
    let prefix = format!("{db}-");
    let mut all: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = fs::read_dir(out) {
        for ent in rd.flatten() {
            let path = ent.path();
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !name.starts_with(&prefix) || !name.ends_with(".dump") {
                continue;
            }
            // bash: [ -f "$_f" ] — skip dirs / dangling
            if path.is_file() {
                all.push(path);
            }
        }
    }
    if all.len() > 1 {
        all.sort_by(|a, b| {
            let sa = a.to_string_lossy();
            let sb = b.to_string_lossy();
            sb.cmp(&sa) // reverse C sort
        });
    }

    if all.is_empty() {
        warn(&format!(
            "retention found no dumps matching {db}-*.dump — expected at least the one just written."
        ));
        return Ok(());
    }

    if all[0] != final_path {
        warn(&format!(
            "newest dump on disk is {}, not the one just written ({}) — skipping prune.",
            all[0].display(),
            final_path.display()
        ));
        return Ok(());
    }

    let keep_usize = keep as usize;
    if all.len() > keep_usize {
        let mut pruned = 0u64;
        for path in all.iter().skip(keep_usize) {
            if path == final_path {
                continue;
            }
            if fs::remove_file(path).is_ok() {
                pruned += 1;
            }
        }
        info(&format!(
            "retention  kept newest {keep}, removed {pruned} older dump(s)"
        ));
    } else {
        info(&format!(
            "retention  {}/{keep} dumps on disk, nothing to prune",
            all.len()
        ));
    }
    Ok(())
}

fn utc_stamp() -> String {
    // Match bash `date -u +%Y%m%dT%H%M%SZ` exactly (same tool, same format).
    let out = Command::new("date")
        .args(["-u", "+%Y%m%dT%H%M%SZ"])
        .output()
        .expect("date");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct PartGuard {
    path: PathBuf,
}

impl Drop for PartGuard {
    fn drop(&mut self) {
        if self.path.is_file() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nonneg_accepts_digits() {
        // die() process::exits on bad input — happy-path only (no invalid/exit cases).
        assert_eq!(parse_nonneg("14", "--keep"), 14);
    }

    #[test]
    fn parse_nonneg_uses_argument() {
        // Would pass both if parse_nonneg ignored `raw` and returned a constant.
        assert_eq!(parse_nonneg("0", "--min-rows"), 0);
        assert_eq!(parse_nonneg("7", "--keep"), 7);
        assert_ne!(parse_nonneg("3", "--keep"), parse_nonneg("9", "--keep"));
    }

    #[test]
    fn retention_sort_is_reverse_lexicographic() {
        let mut all = [
            PathBuf::from("/tmp/tbd_x-20260101T000000Z.dump"),
            PathBuf::from("/tmp/tbd_x-20260201T000000Z.dump"),
            PathBuf::from("/tmp/tbd_x-20251201T000000Z.dump"),
        ];
        all.sort_by(|a, b| b.to_string_lossy().cmp(&a.to_string_lossy()));
        assert_eq!(all[0], PathBuf::from("/tmp/tbd_x-20260201T000000Z.dump"));
    }
}
