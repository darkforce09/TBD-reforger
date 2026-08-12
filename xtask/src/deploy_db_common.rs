//! T-884 — port of `scripts/deploy/lib/db-common.sh` → `cargo xtask deploy db …`.
//!
//! Shared plumbing for backup-db / restore-db / backup-drill (siblings T-885…T-887). Ported
//! FIRST so three callers cannot invent three dump-verifiers. Same propagation argument as
//! `gate-grep.sh` / `crates/tbd-gate` (T-853 / T-556).
//!
//! ── Closed fail-opens (measured in the bash header, preserved here) ─────────────────────────
//!
//! - `pg_restore --list` alone is NOT verification — TOC lives at the head; truncated /
//!   mid-file-corrupt dumps still pass `--list`. Check 5 runs `--data-only` and counts COPY rows.
//! - Identity (T-588): `dbname:` header + `_sqlx_migrations` TOC entry before the body read.
//! - T-381 allow-list refuses `tbd_reforger` unless `--confirm` spells the name twice.
//!
//! This module does **not** source `gate-grep.sh` (T-880 parked). `_sqlx_migrations` probes use
//! `tbd_gate::gate::probe_str` instead.
//!
//! Consumer bridge until T-885/886/887: the three scripts `eval` [`emit_bash_fns`] output so they
//! keep calling `tbd_*` names without sourcing the deleted `.sh` library.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use tbd_gate::Pattern;
use tbd_gate::gate;

/// Subcommands under `cargo xtask deploy db`.
#[derive(Subcommand, Debug)]
pub enum DeployDbCmd {
    /// Emit bash function wrappers for backup/restore/drill until those scripts are ported.
    #[command(name = "emit-bash-fns")]
    EmitBashFns,
    /// T-381 restore-target guard (refuses `tbd_reforger` by default).
    #[command(name = "refuse-unsafe")]
    RefuseUnsafe {
        #[arg(long = "db")]
        db: String,
        #[arg(long = "confirm")]
        confirm: Option<String>,
    },
    /// Parse a single ASCII database name out of a postgres URL.
    #[command(name = "database-name-from-url")]
    DatabaseNameFromUrl { url: String },
    /// Exit 0 if the name is an allow-listed scratch DB.
    #[command(name = "is-safe-scratch")]
    IsSafeScratch {
        #[arg(long = "db")]
        db: String,
    },
    /// Fail closed unless the compose container is running.
    #[command(name = "require-container")]
    RequireContainer,
    /// Print the path of a postgres tool inside the container, or die.
    #[command(name = "require-pg-tool")]
    RequirePgTool { tool: String },
    /// Exit 0 if the database exists.
    #[command(name = "database-exists")]
    DatabaseExists {
        #[arg(long = "db")]
        db: String,
    },
    /// Exact live row count across user tables (not `reltuples`).
    #[command(name = "count-rows")]
    CountRows {
        #[arg(long = "db")]
        db: String,
    },
    /// Five-check dump verifier; prints row count on success.
    #[command(name = "verify-dump")]
    VerifyDump {
        #[arg(long = "file")]
        file: PathBuf,
        #[arg(long = "min-rows", default_value_t = 1)]
        min_rows: u64,
        /// Empty string skips the T-588 identity check (and says so on stderr).
        #[arg(long = "expect-db", default_value = "")]
        expect_db: String,
    },
    /// `runtime exec $CONTAINER …` (no TTY — binary-safe).
    #[command(name = "ct", disable_help_flag = true)]
    Ct {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// `runtime exec -i $CONTAINER …` (stdin inherited).
    #[command(name = "ct-i", disable_help_flag = true)]
    CtI {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Guarded pg_restore (T-886 port of scripts/deploy/restore-db.sh).
    #[command(name = "restore")]
    Restore(crate::deploy_db_restore::RestoreArgs),
}

pub fn run(cmd: DeployDbCmd) -> Result<u8> {
    match cmd {
        DeployDbCmd::EmitBashFns => {
            print!("{}", emit_bash_fns());
            Ok(0)
        }
        DeployDbCmd::RefuseUnsafe { db, confirm } => {
            refuse_unsafe_restore_target(&db, confirm.as_deref())?;
            Ok(0)
        }
        DeployDbCmd::DatabaseNameFromUrl { url } => match database_name_from_url(&url) {
            Some(name) => {
                print!("{name}");
                Ok(0)
            }
            None => Ok(1),
        },
        DeployDbCmd::IsSafeScratch { db } => {
            if is_safe_scratch_database_name(&db) {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        DeployDbCmd::RequireContainer => {
            require_container()?;
            Ok(0)
        }
        DeployDbCmd::RequirePgTool { tool } => {
            let path = require_pg_tool(&tool)?;
            print!("{path}");
            Ok(0)
        }
        DeployDbCmd::DatabaseExists { db } => {
            if database_exists(&db)? {
                Ok(0)
            } else {
                Ok(1)
            }
        }
        DeployDbCmd::CountRows { db } => {
            let rows = count_db_rows(&db)?;
            print!("{rows}");
            Ok(0)
        }
        DeployDbCmd::VerifyDump {
            file,
            min_rows,
            expect_db,
        } => {
            let expect = if expect_db.is_empty() {
                None
            } else {
                Some(expect_db.as_str())
            };
            match verify_dump(&file, min_rows, expect) {
                Ok(rows) => {
                    print!("{rows}");
                    Ok(0)
                }
                Err(VerifyFail) => Ok(1),
            }
        }
        DeployDbCmd::Ct { args } => ct_exec(false, &args),
        DeployDbCmd::CtI { args } => ct_exec(true, &args),
        DeployDbCmd::Restore(args) => crate::deploy_db_restore::run(args),
    }
}

// ─────────────────────────── messaging (bash die / info / warn) ───────────────────────────

pub(crate) fn die(msg: &str) -> ! {
    eprintln!("FATAL: {msg}");
    std::process::exit(1);
}

pub(crate) fn warn(msg: &str) {
    eprintln!("WARN: {msg}");
}

pub(crate) fn info(msg: &str) {
    println!("==> {msg}");
}

// ─────────────────────────── container runtime ───────────────────────────

fn db_container() -> String {
    env::var("TBD_DB_CONTAINER").unwrap_or_else(|_| "tbd_reforger_db".into())
}

pub(crate) fn db_user() -> String {
    env::var("TBD_DB_USER").unwrap_or_else(|_| "tbd".into())
}

/// Resolved container runtime argv prefix (`podman`, `docker`, or `distrobox-host-exec …`).
fn resolve_runtime() -> Vec<String> {
    if let Ok(override_rt) = env::var("TBD_CONTAINER_RUNTIME") {
        if override_rt.trim().is_empty() {
            // fall through to discovery
        } else {
            let parts: Vec<String> = override_rt.split_whitespace().map(str::to_string).collect();
            let head = parts.first().map(String::as_str).unwrap_or("");
            if which(head).is_none() {
                die(&format!(
                    "TBD_CONTAINER_RUNTIME='{override_rt}' but '{head}' is not executable."
                ));
            }
            return parts;
        }
    }
    if which("podman").is_some() {
        return vec!["podman".into()];
    }
    if which("docker").is_some() {
        return vec!["docker".into()];
    }
    if which("distrobox-host-exec").is_some() {
        if host_exec_has("podman") {
            return vec!["distrobox-host-exec".into(), "podman".into()];
        }
        if host_exec_has("docker") {
            return vec!["distrobox-host-exec".into(), "docker".into()];
        }
    }
    die(
        "no container runtime. Tried: $TBD_CONTAINER_RUNTIME, podman, docker, distrobox-host-exec {podman,docker}.\n\
       pg_dump does not exist on this host either, so there is no fallback path.\n\
       Refusing to report a successful backup from a tool that cannot run.",
    );
}

fn which(prog: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let p = dir.join(prog);
            if p.is_file() { Some(p) } else { None }
        })
    })
}

fn host_exec_has(tool: &str) -> bool {
    Command::new("distrobox-host-exec")
        .args(["command", "-v", tool])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn require_container() -> Result<()> {
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..])
        .args(["inspect", "-f", "{{.State.Running}}", &container]);
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn '{}' for inspect", runtime.join(" ")))?;
    let out = String::from_utf8_lossy(&output.stdout);
    let out_trim = out.trim();
    let err = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let combined = if err.trim().is_empty() {
            out_trim.to_string()
        } else {
            err.trim().to_string()
        };
        die(&format!(
            "container '{container}' not found by '{}' (rc={}).\n\
       {combined}\n\
       Start it with: make db-up",
            runtime.join(" "),
            output.status.code().unwrap_or(1)
        ));
    }
    if out_trim != "true" {
        die(&format!(
            "container '{container}' exists but is not running (State.Running={out_trim}).\n\
       A backup taken against a stopped database is not a backup. Start it: make db-up"
        ));
    }
    Ok(())
}

pub(crate) fn require_pg_tool(tool: &str) -> Result<String> {
    // bash: path="$(tbd_ct sh -c "command -v $tool" 2>/dev/null)"
    let (rc, stdout, _stderr) = ct_capture(
        false,
        &["sh".into(), "-c".into(), format!("command -v {tool}")],
    )?;
    let path = stdout.trim().to_string();
    if rc != 0 || path.is_empty() {
        let container = db_container();
        die(&format!(
            "'{tool}' is ABSENT inside container '{container}' (rc={rc}).\n\
       This is not a failed backup, it is a backup that never ran. Refusing to report success.\n\
       Expected a postgres image that ships {tool} (postgres:18-alpine has it at /usr/local/bin/{tool})."
        ));
    }
    Ok(path)
}

/// Inherit stdio `exec` (for `ct` / `ct-i` CLI and binary dump streams).
fn ct_exec(interactive: bool, args: &[String]) -> Result<u8> {
    let _ = resolve_runtime(); // die early with the same message if absent
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..]).arg("exec");
    if interactive {
        cmd.arg("-i");
    }
    cmd.arg(&container).args(args);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn '{}' exec {}", runtime.join(" "), container))?;
    Ok(status.code().unwrap_or(1) as u8)
}

/// Capture stdout/stderr from a non-interactive container exec (tools that need parsing).
pub(crate) fn ct_capture(
    interactive_stdin: bool,
    args: &[String],
) -> Result<(i32, String, String)> {
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..]).arg("exec");
    if interactive_stdin {
        cmd.arg("-i");
    }
    cmd.arg(&container).args(args);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn '{}' exec {}", runtime.join(" "), container))?;
    Ok((
        output.status.code().unwrap_or(1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// `exec -i` with binary stdin from `input`, capturing stdout + stderr separately.
pub(crate) fn ct_i_stdin_capture(args: &[String], input: &[u8]) -> Result<(i32, Vec<u8>, String)> {
    let runtime = resolve_runtime();
    let container = db_container();
    let mut cmd = Command::new(&runtime[0]);
    cmd.args(&runtime[1..])
        .arg("exec")
        .arg("-i")
        .arg(&container)
        .args(args);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().with_context(|| {
        format!(
            "failed to spawn '{}' exec -i {}",
            runtime.join(" "),
            container
        )
    })?;
    {
        let mut sink = child.stdin.take().context("child stdin")?;
        // Closed stdin mid-write is the child's business (matches bash pipe behaviour).
        let _ = sink.write_all(input);
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)?;
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_end(&mut stderr)?;
    }
    let status = child.wait()?;
    Ok((
        status.code().unwrap_or(1),
        stdout,
        String::from_utf8_lossy(&stderr).into_owned(),
    ))
}

// ─────────────────────────── T-381 restore target guard ───────────────────────────

/// `postgres://…/rust_it?sslmode=disable` → `Some("rust_it")`. Empty / multi-segment / non-ASCII → None.
pub fn database_name_from_url(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let after_auth = rest.split_once('/')?.1;
    let name = after_auth
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_string();
    if name.is_empty() || name.contains('/') {
        return None;
    }
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some(name)
}

pub fn is_safe_scratch_database_name(name: &str) -> bool {
    if name.is_empty() || name == "tbd_reforger" {
        return false;
    }
    name == "rust_it"
        || name.starts_with("tbd_gate")
        || name.ends_with("_cold")
        || name.ends_with("_it")
        || name.ends_with("_probe")
}

pub(crate) fn refuse_unsafe_restore_target(name: &str, confirm: Option<&str>) -> Result<()> {
    if name.is_empty() {
        die("restore target database name is empty or unparseable.\n\
       Expected a single ASCII name, e.g. --db rust_it");
    }
    if !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        die(&format!(
            "restore target '{name}' is not a plain ASCII database name ([A-Za-z0-9_])."
        ));
    }
    if is_safe_scratch_database_name(name) {
        return Ok(());
    }
    if let Some(c) = confirm
        && !c.is_empty()
        && c == name
    {
        warn(&format!(
            "restoring over NON-scratch database '{name}' — confirmed via --i-understand-this-destroys={name}"
        ));
        return Ok(());
    }
    eprint!(
        "\
───────────────────────────────────────────────────────────────────────
REFUSING to restore into database `{name}` (T-381 allow-list).

  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe

  The live database `tbd_reforger` is never allowed by default — a
  `pg_restore --clean --if-exists` against it DROPS EVERY OBJECT FIRST,
  so a typo here is unrecoverable without another backup.

  This is the same allow-list the integration harness carries at
  apps/website/api/tests/common/mod.rs:87, which already stopped one
  exported TEST_DATABASE_URL from wiping the live database.

  If you genuinely mean it (disaster recovery), name it twice:
    cargo xtask deploy db restore --db {name} --i-understand-this-destroys={name} <dump>
───────────────────────────────────────────────────────────────────────
"
    );
    std::process::exit(1);
}

// ─────────────────────────── dump verification ───────────────────────────

pub(crate) struct VerifyFail;

pub(crate) fn verify_dump(
    file: &Path,
    min_rows: u64,
    expect_db: Option<&str>,
) -> Result<u64, VerifyFail> {
    if !file.is_file() {
        eprintln!(
            "VERIFY FAIL: '{}' does not exist or is not a regular file.",
            file.display()
        );
        return Err(VerifyFail);
    }
    let meta = fs::metadata(file).map_err(|_| {
        eprintln!(
            "VERIFY FAIL: '{}' does not exist or is not a regular file.",
            file.display()
        );
        VerifyFail
    })?;
    let size = meta.len();
    if size == 0 {
        eprintln!(
            "VERIFY FAIL: '{}' is empty (0 bytes). A zero-byte backup is the failure this check exists for.",
            file.display()
        );
        return Err(VerifyFail);
    }

    let mut magic_buf = [0u8; 5];
    {
        let mut f = fs::File::open(file).map_err(|_| VerifyFail)?;
        let n = f.read(&mut magic_buf).unwrap_or(0);
        if n < 5 || &magic_buf != b"PGDMP" {
            let got = od_c_preview(file);
            eprintln!(
                "VERIFY FAIL: '{}' does not start with the PGDMP custom-format magic (got: {got}).",
                file.display()
            );
            eprintln!(
                "             Was it written by `pg_dump -Fc`? A plain-SQL or gzip dump cannot be verified or restored by this tooling."
            );
            return Err(VerifyFail);
        }
    }

    let bytes = fs::read(file).map_err(|_| {
        eprintln!(
            "VERIFY FAIL: '{}' — could not read the archive for verification.",
            file.display()
        );
        VerifyFail
    })?;

    // 3. TOC
    let (list_rc, toc_bytes, _) = ct_i_stdin_capture(
        &["pg_restore".into(), "--list".into()],
        &bytes,
    )
    .map_err(|_| {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --list` could not read the archive table of contents.",
            file.display()
        );
        VerifyFail
    })?;
    let toc = String::from_utf8_lossy(&toc_bytes);
    if list_rc != 0 || toc.is_empty() {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --list` could not read the archive table of contents.",
            file.display()
        );
        return Err(VerifyFail);
    }

    // 4. IDENTITY (T-588)
    if let Some(expect) = expect_db {
        let dbname = toc
            .lines()
            .find_map(|line| {
                let trimmed = line.trim_start();
                trimmed.strip_prefix(';').and_then(|rest| {
                    let rest = rest.trim_start();
                    rest.strip_prefix("dbname:").map(|v| v.trim().to_string())
                })
            })
            .unwrap_or_default();
        if dbname.is_empty() {
            eprintln!(
                "VERIFY FAIL: '{}' — the archive header carries no `dbname:` line, so the source",
                file.display()
            );
            eprintln!(
                "             database cannot be established. Refusing to vouch for an archive whose"
            );
            eprintln!("             identity is unknown; expected '{expect}'.");
            return Err(VerifyFail);
        }
        if dbname != expect {
            eprintln!(
                "VERIFY FAIL: '{}' is a dump of database '{dbname}', but '{expect}' was expected.",
                file.display()
            );
            eprintln!(
                "             This archive is structurally VALID — it is simply the wrong database."
            );
            eprintln!(
                "             Restoring it would `pg_restore --clean` the target and repopulate it"
            );
            eprintln!(
                "             from the wrong source. If you meant this, pass the real source name."
            );
            return Err(VerifyFail);
        }
        let pat = Pattern::literal("_sqlx_migrations");
        match gate::probe_str(&pat, &toc) {
            Ok(true) => {}
            Ok(false) => {
                eprintln!(
                    "VERIFY FAIL: '{}' names database '{dbname}' but its TOC has no `_sqlx_migrations`",
                    file.display()
                );
                eprintln!(
                    "             table. Every TBD-Reforger database carries one; an archive without it is"
                );
                eprintln!(
                    "             not a backup of this platform's schema, whatever it is named."
                );
                return Err(VerifyFail);
            }
            Err(nr) => {
                eprintln!(
                    "VERIFY FAIL: '{}' — the identity check could not RUN (grep status {nr:?}).",
                    file.display()
                );
                eprintln!(
                    "             A check that did not execute is not a pass. Failing closed."
                );
                return Err(VerifyFail);
            }
        }
    } else {
        eprintln!(
            "VERIFY NOTE: no expected database name was given, so '{}' was NOT checked for",
            file.display()
        );
        eprintln!(
            "             database identity — a valid dump of a DIFFERENT database would pass"
        );
        eprintln!("             everything below. Pass a third argument to close that gap.");
    }

    // 5. Full body read + COPY row count
    let (data_rc, data_out, data_err) = ct_i_stdin_capture(
        &[
            "pg_restore".into(),
            "--data-only".into(),
            "-f".into(),
            "-".into(),
        ],
        &bytes,
    )
    .map_err(|_| VerifyFail)?;
    if data_rc != 0 {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --data-only` exited {data_rc} while reading the archive body.",
            file.display()
        );
        eprintln!(
            "             The file is TRUNCATED or CORRUPT. Note that `pg_restore --list` PASSES on both"
        );
        eprintln!("             of those (measured) — this is the check that catches them.");
        for line in data_err.lines() {
            eprintln!("             {line}");
        }
        return Err(VerifyFail);
    }
    let data_text = String::from_utf8_lossy(&data_out);
    let rows = count_copy_rows(&data_text);
    if rows < min_rows {
        eprintln!(
            "VERIFY FAIL: '{}' restored cleanly but contains {rows} data row(s), below the required minimum of {min_rows}.",
            file.display()
        );
        eprintln!(
            "             A schema-only or all-empty archive is not a backup of a live database."
        );
        eprintln!(
            "             (If backing up a genuinely empty database is intended, pass --min-rows 0.)"
        );
        return Err(VerifyFail);
    }
    Ok(rows)
}

/// Port of the bash awk COPY-block row counter.
fn count_copy_rows(data: &str) -> u64 {
    let mut n: u64 = 0;
    let mut inc = false;
    for line in data.lines() {
        if line.starts_with("COPY ") && line.ends_with(" FROM stdin;") {
            inc = true;
            continue;
        }
        if inc && line == "\\." {
            inc = false;
            continue;
        }
        if inc {
            n += 1;
        }
    }
    n
}

fn od_c_preview(file: &Path) -> String {
    // bash: head -c 5 | od -An -c | tr -s ' '
    match Command::new("head")
        .args(["-c", "5"])
        .arg(file)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|head| {
            let od = Command::new("od")
                .args(["-An", "-c"])
                .stdin(head.stdout.unwrap())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()?;
            Ok(od)
        }) {
        Ok(od) => {
            let s = String::from_utf8_lossy(&od.stdout);
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        }
        Err(_) => "<unreadable>".into(),
    }
}

pub(crate) fn count_db_rows(db: &str) -> Result<String> {
    let user = db_user();
    let sql = "\
		SELECT COALESCE(sum(cnt),0) FROM (
			SELECT (xpath('/row/c/text()',
				query_to_xml(format('SELECT count(*) AS c FROM %I.%I', schemaname, relname),
				false, true, '')))[1]::text::bigint AS cnt
			FROM pg_stat_user_tables
		) t;";
    let (rc, stdout, _) = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.into(),
        ],
    )?;
    if rc != 0 {
        bail!("tbd_count_db_rows: psql exited {rc}");
    }
    Ok(stdout.chars().filter(|c| !c.is_whitespace()).collect())
}

pub(crate) fn database_exists(db: &str) -> Result<bool> {
    let user = db_user();
    // bash interpolates $db into SQL; keep the same shape (callers already ASCII-guard names).
    let sql = format!("SELECT 1 FROM pg_database WHERE datname = '{db}';");
    let (rc, stdout, _) = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            "postgres".into(),
            "-tAc".into(),
            sql,
        ],
    )?;
    if rc != 0 {
        return Ok(false);
    }
    let out: String = stdout.chars().filter(|c| !c.is_whitespace()).collect();
    Ok(out == "1")
}

// ─────────────────────────── bash bridge ───────────────────────────

/// Print bash function definitions that forward to `cargo xtask deploy db …`.
///
/// Used by backup-db / backup-drill until those tickets land (restore is T-886 Rust). No new `.sh` library
/// file — keeps `scripts/shell-inventory.txt` at −1 for this slice.
pub fn emit_bash_fns() -> String {
    // Resolve via `cargo run -q` so cargo status lines do not leak into script stderr.
    // die()/exit in the old sourced lib killed THIS shell; a child xtask exit alone would
    // not — wrappers that map to die/exit must `exit` the caller on failure.
    r#"# T-884 bridge — db-common.sh → cargo xtask deploy db (do not add a new .sh lib)
die() { echo "FATAL: $*" >&2; exit 1; }
info() { echo "==> $*"; }
warn() { echo "WARN: $*" >&2; }
_tbd_xtask_db() { cargo run -q -p xtask -- deploy db "$@"; }
tbd_resolve_runtime() { :; } # runtime resolved inside xtask on each call
tbd_require_container() { _tbd_xtask_db require-container || exit $?; }
tbd_require_pg_tool() { _tbd_xtask_db require-pg-tool "$1" || exit $?; }
tbd_database_name_from_url() { _tbd_xtask_db database-name-from-url "$1"; }
tbd_is_safe_scratch_database_name() { _tbd_xtask_db is-safe-scratch --db "$1"; }
tbd_refuse_unsafe_restore_target() { _tbd_xtask_db refuse-unsafe --db "$1" ${2:+--confirm "$2"} || exit $?; }
tbd_verify_dump() {
	local file="$1" min_rows="${2:-1}" expect_db="${3:-}"
	_tbd_xtask_db verify-dump --file "$file" --min-rows "$min_rows" --expect-db "$expect_db"
}
tbd_count_db_rows() { _tbd_xtask_db count-rows --db "$1"; }
tbd_database_exists() { _tbd_xtask_db database-exists --db "$1"; }
tbd_ct() { _tbd_xtask_db ct -- "$@"; }
tbd_ct_i() { _tbd_xtask_db ct-i -- "$@"; }
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_name_from_url_parses_ascii_path() {
        assert_eq!(
            database_name_from_url("postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable")
                .as_deref(),
            Some("rust_it")
        );
        assert_eq!(
            database_name_from_url(
                "postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable"
            )
            .as_deref(),
            Some("tbd_reforger")
        );
        assert!(database_name_from_url("not-a-url").is_none());
        assert!(database_name_from_url("postgres://h/").is_none());
        assert!(database_name_from_url("postgres://h/a/b").is_none());
        assert!(database_name_from_url("postgres://h/weird-name").is_none());
    }

    #[test]
    fn safe_scratch_allow_list_matches_t381() {
        assert!(is_safe_scratch_database_name("rust_it"));
        assert!(is_safe_scratch_database_name("tbd_gate_it"));
        assert!(is_safe_scratch_database_name("tbd_wave6_cold"));
        assert!(is_safe_scratch_database_name("tbd_t350_probe"));
        assert!(is_safe_scratch_database_name("tbd_t230_it"));
        assert!(is_safe_scratch_database_name("tbd_t884_probe"));
        assert!(!is_safe_scratch_database_name("tbd_reforger"));
        assert!(!is_safe_scratch_database_name("postgres"));
        assert!(!is_safe_scratch_database_name(""));
        assert!(!is_safe_scratch_database_name("production"));
    }

    #[test]
    fn count_copy_rows_counts_data_not_headings() {
        let sample = "\
COPY public.t (id) FROM stdin;
1
2
\\.
COPY public.empty (id) FROM stdin;
\\.
";
        assert_eq!(count_copy_rows(sample), 2);
    }
}
