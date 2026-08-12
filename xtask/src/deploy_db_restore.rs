//! T-886 — guarded PostgreSQL restore (`scripts/deploy/restore-db.sh` → `cargo xtask deploy db restore`).
//!
//! Load-bearing: T-381 allow-list (`refuse_unsafe_restore_target`) and verify-before-write
//! (`verify_dump`). Do not reimplement those — call `deploy_db_common`.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::deploy_db_common::{
    VerifyFail, count_db_rows, ct_capture, ct_i_stdin_capture, database_exists,
    database_name_from_url, db_user, die, info, refuse_unsafe_restore_target, require_container,
    require_pg_tool, verify_dump, warn,
};

#[derive(Args, Debug)]
pub struct RestoreArgs {
    /// Target database name.
    #[arg(long = "db")]
    db: Option<String>,

    /// Target as a postgres:// URL (database name parsed from the path).
    #[arg(long = "url")]
    url: Option<String>,

    /// Create the target database first if it does not exist.
    #[arg(long = "create")]
    create: bool,

    /// Parallel restore workers (`pg_restore -j`).
    #[arg(long = "jobs", default_value_t = default_jobs())]
    jobs: u32,

    /// Require the dump to hold >= N data rows before restoring.
    #[arg(long = "min-rows", default_value_t = default_min_rows())]
    min_rows: u64,

    /// Database the dump must have been taken FROM (not the restore target).
    /// Pass empty string to skip the identity check.
    #[arg(long = "expect-db")]
    expect_db: Option<String>,

    /// Required to target a database outside the T-381 allow-list; must equal `--db`.
    #[arg(long = "i-understand-this-destroys")]
    confirm: Option<String>,

    /// Custom-format dump file (`pg_dump -Fc`).
    dump: PathBuf,
}

fn default_jobs() -> u32 {
    env::var("TBD_RESTORE_JOBS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(1)
}

fn default_min_rows() -> u64 {
    env::var("TBD_RESTORE_MIN_ROWS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn default_expect_db() -> String {
    env::var("TBD_RESTORE_EXPECT_DB")
        .or_else(|_| env::var("TBD_BACKUP_DB"))
        .unwrap_or_else(|_| "tbd_reforger".into())
}

pub fn run(args: RestoreArgs) -> Result<u8> {
    if args.jobs < 1 {
        die("--jobs must be >= 1");
    }

    let mut db = args.db.clone().unwrap_or_default();
    if let Some(ref url) = args.url {
        if !db.is_empty() {
            die(&format!(
                "pass --db or --url, not both (got db='{db}' url='{url}')."
            ));
        }
        db = database_name_from_url(url).unwrap_or_else(|| {
            die(&format!(
                "could not parse a database name out of --url '{url}'.\n\
       Expected e.g. postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable"
            ));
        });
    }
    if db.is_empty() {
        eprintln!("One of --db or --url is required.");
        eprintln!();
        eprintln!("{}", usage_blurb());
        return Ok(2);
    }

    // ── THE GUARD. Before the container is even contacted. ──────────────────────────────
    refuse_unsafe_restore_target(&db, args.confirm.as_deref())?;

    require_container()?;
    let _ = require_pg_tool("pg_restore")?;
    let _ = require_pg_tool("psql")?;

    let expect_owned = args.expect_db.clone().unwrap_or_else(default_expect_db);
    let expect = if expect_owned.is_empty() {
        None
    } else {
        Some(expect_owned.as_str())
    };

    // ── VERIFY THE ARCHIVE BEFORE DROPPING ANYTHING ─────────────────────────────────────
    info(&format!(
        "verifying {} before touching '{db}'",
        args.dump.display()
    ));
    let rows = match verify_dump(&args.dump, args.min_rows, expect) {
        Ok(r) => r,
        Err(VerifyFail) => {
            eprintln!(
                "FAIL: refusing to restore — the archive did not verify. Database '{db}' is UNTOUCHED."
            );
            eprintln!(
                "      `--clean` drops before it restores, so restoring an unreadable archive would"
            );
            eprintln!("      have destroyed '{db}' and put nothing back.");
            return Ok(1);
        }
    };
    info(&format!("archive OK — {rows} data row(s)"));

    if !database_exists(&db)? {
        if args.create {
            info(&format!("creating database '{db}'"));
            let user = db_user();
            let sql = format!("CREATE DATABASE \"{db}\";");
            let (rc, _, err) = ct_capture(
                false,
                &[
                    "psql".into(),
                    "-U".into(),
                    user,
                    "-d".into(),
                    "postgres".into(),
                    "-c".into(),
                    sql,
                ],
            )?;
            if rc != 0 {
                if !err.trim().is_empty() {
                    eprintln!("{err}");
                }
                die(&format!("could not create database '{db}'"));
            }
        } else {
            die(&format!(
                "database '{db}' does not exist. Pass --create to create it first."
            ));
        }
    }

    info(&format!(
        "restoring into '{db}' (pg_restore --clean --if-exists -j {})",
        args.jobs
    ));
    // --exit-on-error so a restore that hit errors cannot report success; pg_restore's default
    // is to continue and exit 0, which is the fail-open shape this whole ticket is about.
    // NOTE: --single-transaction is incompatible with -j>1, so it is only used for -j 1.
    // (bash never actually passed --single-transaction; keep that shape.)
    let user = db_user();
    let mut restore_args = vec![
        "pg_restore".into(),
        "--clean".into(),
        "--if-exists".into(),
        "--no-owner".into(),
        "--no-privileges".into(),
        "--exit-on-error".into(),
        "-U".into(),
        user,
        "-d".into(),
        db.clone(),
    ];
    if args.jobs > 1 {
        restore_args.push("-j".into());
        restore_args.push(args.jobs.to_string());
    }

    let dump_bytes = fs::read(&args.dump)
        .map_err(|e| anyhow::anyhow!("could not read dump '{}': {e}", args.dump.display()))?;
    let (rc, stdout, stderr) = ct_i_stdin_capture(&restore_args, &dump_bytes)?;
    let combined = {
        let mut s = String::from_utf8_lossy(&stdout).into_owned();
        if !stderr.is_empty() {
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(&stderr);
        }
        s
    };
    if rc != 0 {
        eprintln!("FAIL: pg_restore exited {rc}. Database '{db}' may be in a partial state.");
        for line in combined
            .lines()
            .rev()
            .take(40)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            eprintln!("      {line}");
        }
        return Ok(1);
    }
    if !combined.trim().is_empty() {
        warn("pg_restore output:");
        for line in combined
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            eprintln!("      {line}");
        }
    }

    let got = count_db_rows(&db)?;
    info(&format!(
        "restored   '{db}' now reports {got} live row(s) (archive held {rows})"
    ));
    if !got.is_empty() {
        if let Ok(got_n) = got.parse::<u64>() {
            if got_n == 0 && rows > 0 {
                eprintln!("FAIL: archive held {rows} rows but '{db}' reports 0 after restore.");
                return Ok(1);
            }
        }
    }
    info("done");
    Ok(0)
}

fn usage_blurb() -> &'static str {
    "Usage: cargo xtask deploy db restore (--db NAME | --url URL) [options] <dump-file>\n\
\n\
  Verify a custom-format dump, then restore it with `pg_restore --clean --if-exists`.\n\
\n\
  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe\n\
  Refused by default:           tbd_reforger and everything else"
}
