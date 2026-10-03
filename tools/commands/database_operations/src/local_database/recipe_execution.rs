//! How the database lane runs a recipe: echo the line, resolve the folder and runtime, spawn,
//! report the child's exit status.
//!
//! **Role:** the API folder and container runtime the lane's recipes run against, the make-style
//! echo of each recipe line, the honest child exit status, and the three recipes that spawn
//! straight to the terminal: the compose lane (`db up` / `db down` / `db logs`), `db seed` and
//! `db registry-import`.
//! **Position:** a child of [`crate::local_database`], whose `run` dispatch calls [`compose`],
//! [`seed`] and [`registry_import`]; [`super::test_it`] and [`super::repair_migration_checksum`]
//! reuse [`web`], [`runtime`], [`echo`] and [`finish_status`]; [`super::recipes`] renders the seed
//! line from [`seed_psql_arguments`].
//! **Signals & state:** none; every call resolves the repository root and the runtime afresh and
//! owns its child until it exits.
//! **Invariants:** every echoed line is flushed before its child starts, so output interleaves as
//! make's did; the echoed runtime is the logical name, never the bridge prefix; a signalled child
//! is reported as a signal, never folded into an exit code.
//!
//! ── THE BRIDGE: `podman` DOES NOT EXIST IN THE AGENT CONTAINER ───────────────────────────────
//!
//! MEASURED 2026-08-12 inside `claude-desktop` (debian:12, `/run/.containerenv` present):
//!
//! ```text
//! $ make db-up
//! cd apps/api && podman compose up -d db
//! /bin/sh: 1: podman: not found
//! make: *** [Makefile:70: db-up] Error 127
//! ```
//!
//! `podman`, `docker`, `psql` and `pg_dump` are **all absent in-container**; the container runtime
//! lives on the host, and the only way to it is `distrobox-host-exec`. Every compose/psql target
//! here therefore crosses the bridge. It does NOT reimplement it: [`crate::container_database::
//! resolve_runtime`] already resolves `podman` → `docker` → `distrobox-host-exec {podman,docker}`,
//! and [`process_runner::host_execution`] documents why presence of `distrobox-host-exec` is not a container test
//! (it is installed on the host too, where it refuses with exit 126).
//!
//! Consequence worth stating plainly: **five of these targets are simply broken in-container today
//! and work after this port.** That is the point of the slice, and it is also why a byte-diff of
//! `make` against the port has to be taken with `make` running on the side of the bridge where it
//! can work (see `crate::local_database::selftest`).

use std::io::Write;
use std::path::PathBuf;

use process_runner::Run;
use verification_core::NotRun;

use super::development_compose::{ComposeProject, compose_argv};
use super::{IT_MAINT_DB, SEEDS, WEB, seed_file};
use crate::container_database as dbc;
use crate::error::{Result, ResultExt};
use repository_layout::find_repository_root;

// ── THE API FOLDER AND THE CONTAINER RUNTIME ────────────────────────────────────────────────────

/// The API crate folder ([`WEB`]) as the echoed lines name it, alongside the absolute path the
/// child runs in: `db test-it` and `db registry-import` run cargo there.
pub(crate) struct Web {
    pub(crate) rel: String,
    pub(crate) abs: PathBuf,
}

/// The API crate folder of this checkout.
pub(crate) fn web() -> Result<Web> {
    let root = find_repository_root()?;
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
pub(crate) fn runtime() -> Result<(Vec<String>, String)> {
    let rt = dbc::resolve_runtime()?;
    let logical = rt.last().cloned().unwrap_or_else(|| "podman".to_string());
    Ok((rt, logical))
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

/// Child rc, honestly: a signalled child is reported as such instead of being folded into 128+n,
/// and a child that never ran is an error under the context `context` builds.
pub(crate) fn finish_status<C: std::fmt::Display>(
    label: &str,
    outcome: Result<i32, NotRun>,
    context: impl FnOnce() -> C,
) -> Result<u8> {
    match outcome {
        Ok(c) => Ok(c.clamp(0, 255) as u8),
        Err(NotRun::Signalled { signal, .. }) => {
            eprintln!(
                "xtask db: `{label}` was killed by signal {signal} — the process died, it did not report."
            );
            Ok(1)
        }
        Err(cause) => Err(cause).with_context(context),
    }
}

// ── COMPOSE LANE: db-up / db-down / db-logs / seed ───────────────────────────────────────────

/// `cd <folder> && <runtime> compose -f <file> <args> [< stdin]` — the whole compose lane in one
/// shape. [`ComposeProject`] resolves the folder and file and renders the line; `stdin_from_root`
/// is a repository-relative file read on stdin.
pub(super) fn compose(args: &[&str], stdin_from_root: Option<&str>) -> Result<u8> {
    let project = ComposeProject::resolve()?;
    let (rt, logical) = runtime()?;
    echo(&project.shown.render(&logical, args, stdin_from_root));

    let argv = compose_argv(&rt, args);
    trace(&argv);

    // On the inherited terminal: `db logs -f` streams until interrupted, and a seed file is read
    // on stdin.
    let mut child = Run::new(&argv[0]).args(&argv[1..]).cwd(&project.folder);
    child = match stdin_from_root {
        // The line's shell redirect, opened directly: the same path, resolved against the folder.
        // A missing file is a shell's "cannot open …" rc 2 — same rc, honest text (no shell ran).
        Some(from_root) => {
            let path = project.stdin_file(from_root);
            match std::fs::File::open(&path) {
                Ok(f) => child.stdin_file(f),
                Err(e) => {
                    eprintln!("xtask db: cannot open {}: {e}", path.display());
                    return Ok(2);
                }
            }
        }
        None => child.stdin_null(),
    };
    finish_status(&argv.join(" "), child.terminal(), || {
        format!("failed to spawn '{}'", argv.join(" "))
    })
}

/// The `compose` arguments that apply one seed file read on stdin: `psql` in the `db` service,
/// with `ON_ERROR_STOP` set so a failed statement ends the file with a non-zero exit instead of
/// `psql` carrying on and exiting 0.
pub(crate) fn seed_psql_arguments() -> Vec<String> {
    let command = format!("exec -T db psql -v ON_ERROR_STOP=1 -U tbd -d {IT_MAINT_DB}");
    command.split(' ').map(str::to_string).collect()
}

/// Applies the [`SEEDS`] in order and stops at the first file whose `psql` run fails.
pub(super) fn seed() -> Result<u8> {
    let arguments = seed_psql_arguments();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    for file in SEEDS {
        let rc = compose(&arguments, Some(&seed_file(file)))?;
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
pub(super) fn registry_import() -> Result<u8> {
    let web = web()?;
    const ITEMS: &str = "../../contracts/catalogs/registry-items.workbench.json";
    const COMPAT: &str = "../../contracts/catalogs/registry-compat.workbench.json";
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
    // On the inherited terminal: `cargo run` streams its build and import output as it goes.
    let outcome = Run::new(&argv[0]).args(&argv[1..]).cwd(&web.abs).terminal();
    finish_status(
        "cargo run --bin import-registry",
        outcome,
        || "failed to spawn cargo run --bin import-registry",
    )
}
