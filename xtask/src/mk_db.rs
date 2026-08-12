//! T-894 — the Makefile's **database lane** as `cargo xtask db …`.
//!
//! T-853 Phase 3 replaces `make` with `cargo xtask`. This module is the first of the three build
//! slices (T-894 db / T-895 build / T-896 ci); T-897 deletes the Makefile afterwards. **This slice
//! does not touch the Makefile** — the three build slices stay file-disjoint on purpose, so the
//! only thing that lands here is the successor plus the evidence that it matches.
//!
//! Targets absorbed (Makefile:69-113, 201-212):
//!
//! | make | xtask |
//! |---|---|
//! | `db-up` `db-down` `db-logs` `seed` | `db up` `db down` `db logs` `db seed` |
//! | `db-backup` `db-restore` `db-backup-drill` `db-backup-verify` | `db backup` `db restore` `db backup-drill` `db backup-verify` |
//! | `registry-import` | `db registry-import` |
//! | `rust-test-it` | `db test-it` |
//!
//! ── THE BRIDGE: `podman` DOES NOT EXIST IN THE AGENT CONTAINER ───────────────────────────────
//!
//! MEASURED 2026-08-12 inside `claude-desktop` (debian:12, `/run/.containerenv` present):
//!
//! ```text
//! $ make db-up
//! cd apps/website/api && podman compose up -d db
//! /bin/sh: 1: podman: not found
//! make: *** [Makefile:70: db-up] Error 127
//! ```
//!
//! `podman`, `docker`, `psql` and `pg_dump` are **all absent in-container**; the container runtime
//! lives on the host, and the only way to it is `distrobox-host-exec`. Every compose/psql target
//! here therefore crosses the bridge. It does NOT reimplement it: [`crate::deploy_db_common::
//! resolve_runtime`] already resolves `podman` → `docker` → `distrobox-host-exec {podman,docker}`,
//! and [`crate::hostrun`] documents why presence of `distrobox-host-exec` is not a container test
//! (it is installed on the host too, where it refuses with exit 126).
//!
//! Consequence worth stating plainly: **five of these targets are simply broken in-container today
//! and work after this port.** That is the point of the slice, and it is also why a byte-diff of
//! `make` against the port has to be taken with `make` running on the side of the bridge where it
//! can work (see `mk_db::selftest`).
//!
//! ── WHAT "BYTE-IDENTICAL" MEANS HERE, EXACTLY ────────────────────────────────────────────────
//!
//! `make` echoes each recipe line to stdout before running it. That echo is part of the observed
//! output, so the port reproduces it **verbatim** for every target whose recipe shells out — same
//! text, same order, flushed before the child starts so interleaving matches.
//!
//! Two deliberate divergences, both narrow and both louder rather than quieter:
//!
//! 1. **The four `deploy db` wrappers echo nothing.** `db-backup`/`db-restore`/`db-backup-drill`/
//!    `db-backup-verify` are one-line recipes that shell out to `cargo run -q -p xtask -- deploy
//!    db …` (T-884…T-887 already ported that lane). This module calls those Rust functions
//!    **in-process**, so echoing `cargo run -q -p xtask -- …` would be a fabricated trace of a
//!    command that never ran. The make side prints exactly one extra line — its own transport —
//!    and the argv mapping (including make's `$(if $(DB),--db $(DB),)` conditionals, which expand
//!    to *trailing spaces* when unset: `cargo run -q -p xtask -- deploy db backup   `) is pinned
//!    by [`selftest`]'s recipe arm instead.
//! 2. **Exit status is the child's, not make's.** A failed recipe makes `make` print
//!    `make: *** [Makefile:70: db-up] Error 127` and exit **2**, folding every rc into one. The
//!    port returns the child's raw rc (127 stays 127) and reports a signalled child as a signal —
//!    the `tbd_gate::proc` thesis. Nothing consumes make's `Error N` line; several things would
//!    like the real rc.
//!
//! ── ODDITIES PRESERVED (reproduce, pin, document — not silently "improved") ───────────────────
//!
//! - `seed:` applies five SQL files in a fixed order; `gate_t444` pins the wiki entry to that
//!   recipe. [`SEEDS`] keeps the order and the [`selftest`] recipe arm keeps them in step.
//!   **T-897 must repoint `gate_t444`'s `RECIPE_FILE`/`RECIPE_TARGET` at [`SEEDS`]** or that
//!   Class-R gate evaporates with the Makefile.
//! - `rust-test-it` hardcodes `podman` (not `$(COMPOSE)`), the container name `tbd_reforger_db`,
//!   the user `tbd`, the maintenance database `tbd_reforger`, and the URL `localhost:5434`. The
//!   port keeps all five as defaults but routes the runtime/container/user through
//!   `deploy_db_common`, so a docker-only or renamed-container host works instead of failing 127.
//! - The leading `-` on the first `DROP DATABASE` (make ignores that line's status) and the `@` on
//!   the reap block (silent) are both preserved.
//! - `make db-restore` with no `DUMP=`/`DB=` prints a usage line naming *make* and exits 2. The
//!   port prints that byte-for-byte, including the now-wrong `make db-restore …` spelling: rc AND
//!   text parity is the acceptance criterion for this slice, and rewording it is T-897's call.
//!
//! ── FAIL-OPENS CLOSED, AND ONE LATENT BUG FIXED ──────────────────────────────────────────────
//!
//! - **The reap loop reported success when it reaped nothing.** `psql … | while read -r db; …;
//!   done` takes the rc of the *loop*, which is 0 when the pipeline's left side dies. With the DB
//!   container down, `make rust-test-it`'s last line is GREEN while every leftover database
//!   survives. Measured, and pinned as the RED-arm evidence in [`selftest`]. The port checks the
//!   query's rc, prints psql's stderr, and fails.
//! - **A stray base name could reach `DROP DATABASE`.** The Makefile has no knob, so its literal
//!   `rust_it` is safe by construction; the port adds `TBD_IT_BASE_DB` (the selftest needs a
//!   scratch base to avoid racing sibling slices), which would be a loaded gun without a guard. It
//!   goes through the SAME T-381 allow-list the integration harness carries
//!   (`apps/website/api/tests/common/mod.rs:87` ⇄ [`crate::deploy_db_common::
//!   is_safe_scratch_database_name`]), and every individual name is re-checked immediately before
//!   its `DROP`. `tbd_reforger` is refused twice over.
//! - **The reap was skipped on exactly the runs that leak.** `cargo test` failing aborts the make
//!   recipe before the T-558 prune, so a red suite leaves its `rust_it_<suite>_it` databases
//!   behind — the case the prune exists for. The port always reaps and then returns the test rc.
//!   Output is unchanged (the reap is silent by design), so this costs no parity.
//! - No container runtime at all is a `FATAL:` refusal from `resolve_runtime()` rather than a
//!   `sh: podman: not found` 127.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::deploy_db_common as dbc;
use crate::root::find_repo_root;

pub mod ab;
pub mod recipes;
pub mod selftest;
pub mod test_it;

// ── THE RECIPES, FROZEN ──────────────────────────────────────────────────────────────────────
// These consts are the port's single source of truth AND the baseline captured from the Makefile
// on 2026-08-12. `selftest`'s recipe arm re-derives them from the live Makefile while it exists;
// after T-897 deletes it, the consts are what survives — which is why the pin lives here and not
// only in a test that reads a file that is going away.

/// `WEB := apps/website/api` (Makefile:3).
pub(crate) const WEB: &str = "apps/website/api";

/// `seed:` — five appliers, in order (Makefile:78-83). Order is contractual: `registry_dev`
/// references roles seeded by `discord_roles`.
pub(crate) const SEEDS: &[&str] = &[
    "discord_roles.sql",
    "registry_dev.sql",
    "faction_library.sql",
    "vehicle_database.sql",
    "wiki_pages.sql",
];

/// `rust-test-it`'s base database (Makefile:205-207, literal).
pub(crate) const IT_BASE_DB: &str = "rust_it";
/// The maintenance database `psql` connects to in order to drop/create the scratch one — the LIVE
/// dev database, per the Makefile. Preserved; nothing is written to it.
pub(crate) const IT_MAINT_DB: &str = "tbd_reforger";
/// `make db-restore` / `make db-backup-verify` usage lines (Makefile:99-100, 107). Frozen text.
const USAGE_RESTORE: &str = "usage: make db-restore DUMP=<file.dump> DB=<target> [CREATE=1]";
const USAGE_VERIFY: &str = "usage: make db-backup-verify DUMP=<file.dump>";

/// Subcommands under `cargo xtask db`.
#[derive(Subcommand, Debug)]
pub enum DbCmd {
    /// `make db-up` — start local Postgres in the background.
    Up,
    /// `make db-down` — stop local Postgres (keeps the data volume).
    Down,
    /// `make db-logs` — tail the Postgres logs (`-f`, runs until interrupted).
    Logs,
    /// `make seed` — apply the five data seeds to the running DB.
    Seed,
    /// `make db-backup` — verified dump + prune (T-885 lane).
    Backup {
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        keep: Option<String>,
    },
    /// `make db-restore DUMP=… DB=…` — verify a dump then `pg_restore --clean` (T-381 allow-list).
    Restore {
        #[arg(long)]
        dump: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        create: bool,
    },
    /// `make db-backup-drill` — restore the newest backup into a scratch DB and prove it boots.
    #[command(name = "backup-drill")]
    BackupDrill {
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        fresh: bool,
    },
    /// `make db-backup-verify DUMP=…` — re-verify an existing dump without taking a new one.
    #[command(name = "backup-verify")]
    BackupVerify {
        #[arg(long)]
        dump: Option<String>,
    },
    /// `make registry-import` — ingest the committed T-150 registry envelopes into the dev DB.
    #[command(name = "registry-import")]
    RegistryImport,
    /// `make rust-test-it` — fresh `rust_it` DB, run the suite, reap the per-binary leftovers.
    #[command(name = "test-it")]
    TestIt,
    /// T-556 acceptance harness: bash-vs-port arms, each with its own RED proof.
    Selftest,
}

pub fn run(cmd: DbCmd) -> Result<u8> {
    match cmd {
        DbCmd::Up => compose(&["up", "-d", "db"], None),
        DbCmd::Down => compose(&["down"], None),
        DbCmd::Logs => compose(&["logs", "-f", "db"], None),
        DbCmd::Seed => seed(),
        DbCmd::Backup { db, out, keep } => {
            // Makefile:96 — `$(if $(DB),--db $(DB),)` etc. Unset vars contribute nothing.
            let mut argv = Vec::new();
            push_opt(&mut argv, "--db", db.as_deref());
            push_opt(&mut argv, "--out", out.as_deref());
            push_opt(&mut argv, "--keep", keep.as_deref());
            crate::deploy_db_backup::run(&argv)
        }
        DbCmd::Restore { dump, db, create } => restore(dump.as_deref(), db.as_deref(), create),
        DbCmd::BackupDrill { db, out, fresh } => {
            let mut argv = Vec::new();
            push_opt(&mut argv, "--db", db.as_deref());
            push_opt(&mut argv, "--out", out.as_deref());
            if fresh {
                argv.push("--fresh".to_string());
            }
            crate::deploy_db_drill::run(&argv)
        }
        DbCmd::BackupVerify { dump } => match dump.as_deref() {
            None | Some("") => {
                println!("{USAGE_VERIFY}");
                Ok(2)
            }
            Some(d) => crate::deploy_db_backup::run(&["--verify-only".to_string(), d.to_string()]),
        },
        DbCmd::RegistryImport => registry_import(),
        DbCmd::TestIt => test_it::run(),
        DbCmd::Selftest => selftest::run(),
    }
}

fn push_opt(argv: &mut Vec<String>, flag: &str, val: Option<&str>) {
    if let Some(v) = val.filter(|v| !v.is_empty()) {
        argv.push(flag.to_string());
        argv.push(v.to_string());
    }
}

// ── $(WEB) AND $(COMPOSE) ────────────────────────────────────────────────────────────────────

/// `$(WEB)` as make would echo it, alongside the absolute path the child actually runs in.
pub(crate) struct Web {
    pub(crate) rel: String,
    pub(crate) abs: PathBuf,
}

/// `TBD_MK_WEB` is this port's stand-in for `make db-up WEB=<dir>`: the acceptance arms need a
/// throwaway compose project so `db down` in an A/B run cannot stop the `tbd_reforger_db` that
/// sibling slices are testing against. Same precedent as `TBD_FETCH_ROOT` in `gate_fetch_*`.
/// Relative values resolve against the CWD, matching `make`'s own root-relative `cd $(WEB)`.
pub(crate) fn web() -> Result<Web> {
    if let Some(v) = std::env::var_os("TBD_MK_WEB").filter(|v| !v.is_empty()) {
        let rel = v.to_string_lossy().into_owned();
        let p = PathBuf::from(&rel);
        let abs = if p.is_absolute() {
            p
        } else {
            std::env::current_dir().context("cwd")?.join(p)
        };
        return Ok(Web { rel, abs });
    }
    let root = find_repo_root()?;
    Ok(Web {
        rel: WEB.to_string(),
        abs: root.join(WEB),
    })
}

/// The `$(COMPOSE)`/`podman` spelling as make would print it — i.e. WITHOUT the bridge prefix.
///
/// Makefile:2 picks `docker compose` when `docker` is on PATH, else `podman compose`. Here the
/// choice is `resolve_runtime()`'s (podman → docker → bridge), and the echoed name is the last
/// element of its argv: `["distrobox-host-exec", "podman"]` prints as `podman`, because the bridge
/// is transport, not the command. A reader who needs the real argv sets `TBD_MK_TRACE=1`.
pub(crate) fn runtime() -> (Vec<String>, String) {
    let rt = dbc::resolve_runtime();
    let logical = rt.last().cloned().unwrap_or_else(|| "podman".to_string());
    (rt, logical)
}

/// Echo a recipe line exactly as make does, flushed so the child's output lands after it.
pub(crate) fn echo(line: &str) {
    println!("{line}");
    let _ = std::io::stdout().flush();
}

fn trace(argv: &[String]) {
    if std::env::var_os("TBD_MK_TRACE").is_some() {
        eprintln!("+ {}", argv.join(" "));
    }
}

/// Child rc, honestly: a signalled child is reported as such instead of being folded into 128+n.
pub(crate) fn finish_status(label: &str, st: std::process::ExitStatus) -> u8 {
    use std::os::unix::process::ExitStatusExt;
    match st.code() {
        Some(c) => c.clamp(0, 255) as u8,
        None => {
            let sig = st.signal().unwrap_or(0);
            eprintln!(
                "xtask db: `{label}` was killed by signal {sig} — the process died, it did not report."
            );
            1
        }
    }
}

// ── COMPOSE LANE: db-up / db-down / db-logs / seed ───────────────────────────────────────────

/// `cd $(WEB) && $(COMPOSE) <args> [< redirect]` — the whole compose lane in one shape.
fn compose(args: &[&str], redirect_in: Option<&str>) -> Result<u8> {
    let web = web()?;
    let (rt, logical) = runtime();
    let tail = args.join(" ");
    let suffix = redirect_in.map(|f| format!(" < {f}")).unwrap_or_default();
    echo(&format!(
        "cd {web_rel} && {logical} compose {tail}{suffix}",
        web_rel = web.rel
    ));

    let mut argv: Vec<String> = rt.clone();
    argv.push("compose".into());
    argv.extend(args.iter().map(|a| a.to_string()));
    trace(&argv);

    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]).current_dir(&web.abs);
    match redirect_in {
        // The redirect is make's shell doing `< seeds/x.sql`; here the file is opened directly.
        // A missing file is dash's "cannot open …" rc 2 — same rc, honest text (no shell ran).
        Some(rel) => {
            let path = web.abs.join(rel);
            match std::fs::File::open(&path) {
                Ok(f) => {
                    cmd.stdin(Stdio::from(f));
                }
                Err(e) => {
                    eprintln!("xtask db: cannot open {}: {e}", path.display());
                    return Ok(2);
                }
            }
        }
        None => {
            cmd.stdin(Stdio::null());
        }
    }
    let st = cmd
        .status()
        .with_context(|| format!("failed to spawn '{}'", argv.join(" ")))?;
    Ok(finish_status(&argv.join(" "), st))
}

/// `make seed` — five appliers; make aborts at the first failing line, so this does too.
fn seed() -> Result<u8> {
    for file in SEEDS {
        let rc = compose(
            &["exec", "-T", "db", "psql", "-U", "tbd", "-d", IT_MAINT_DB],
            Some(&format!("seeds/{file}")),
        )?;
        if rc != 0 {
            return Ok(rc);
        }
    }
    Ok(0)
}

// ── registry-import ──────────────────────────────────────────────────────────────────────────

/// `make registry-import` (Makefile:110-113) — a three-line backslash continuation, echoed by make
/// with its backslashes and leading tabs intact. The port runs the same argv in the same cwd, so
/// the echo is reproduced exactly, tabs included.
fn registry_import() -> Result<u8> {
    let web = web()?;
    const ITEMS: &str = "../../../packages/tbd-schema/registry/registry-items.workbench.json";
    const COMPAT: &str = "../../../packages/tbd-schema/registry/registry-compat.workbench.json";
    echo(&format!(
        "cd {} && cargo run --bin import-registry -- \\\n\t--items {ITEMS} \\\n\t--compat {COMPAT}",
        web.rel
    ));
    let argv: Vec<String> = vec![
        "cargo".into(),
        "run".into(),
        "--bin".into(),
        "import-registry".into(),
        "--".into(),
        "--items".into(),
        ITEMS.into(),
        "--compat".into(),
        COMPAT.into(),
    ];
    trace(&argv);
    let st = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&web.abs)
        .status()
        .context("failed to spawn cargo run --bin import-registry")?;
    Ok(finish_status("cargo run --bin import-registry", st))
}

// ── db-restore ───────────────────────────────────────────────────────────────────────────────

/// Makefile:98-101. Both guards fire before anything runs, in DUMP-then-DB order, printing to
/// stdout (make's `echo`) and exiting 2.
fn restore(dump: Option<&str>, db: Option<&str>, create: bool) -> Result<u8> {
    if dump.unwrap_or("").is_empty() || db.unwrap_or("").is_empty() {
        println!("{USAGE_RESTORE}");
        return Ok(2);
    }
    let mut argv = vec![
        "restore".to_string(),
        "--db".to_string(),
        db.unwrap_or("").to_string(),
    ];
    if create {
        argv.push("--create".to_string());
    }
    argv.push(dump.unwrap_or("").to_string());
    // Parse through clap so the defaults (`--jobs`, `--min-rows`) are the ones the Makefile's
    // `cargo run … deploy db restore` would have got — not a hand-built struct that drifts.
    #[derive(Parser)]
    struct Argv {
        #[command(flatten)]
        inner: crate::deploy_db_restore::RestoreArgs,
    }
    let parsed = Argv::parse_from(argv);
    crate::deploy_db_restore::run(parsed.inner)
}
